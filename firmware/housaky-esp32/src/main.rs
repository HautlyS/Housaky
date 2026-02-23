//! Housaky ESP32 firmware — JSON-over-serial peripheral.
//!
//! Listens for newline-delimited JSON commands on UART0, executes gpio_read/gpio_write,
//! responds with JSON. Compatible with host Housaky SerialPeripheral protocol.
//!
//! Protocol: same as STM32 — see docs/hardware-peripherals-design.md

use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::uart::*;
use heapless::sync::Mutex;
use log::info;
use serde::{Deserialize, Serialize};

const INPUT_PINS: [i32; 12] = [0, 1, 3, 4, 5, 12, 14, 15, 16, 17, 18, 19];

static INPUT_PINS_STATE: Mutex<
    Option<(
        PinDriver<'static, esp_idf_svc::hal::gpio::Gpio0, esp_idf_svc::hal::gpio::Input>,
        PinDriver<'static, esp_idf_svc::hal::gpio::Gpio1, esp_idf_svc::hal::gpio::Input>,
        PinDriver<'static, esp_idf_svc::hal::gpio::Gpio3, esp_idf_svc::hal::gpio::Input>,
        PinDriver<'static, esp_idf_svc::hal::gpio::Gpio4, esp_idf_svc::hal::gpio::Input>,
        PinDriver<'static, esp_idf_svc::hal::gpio::Gpio5, esp_idf_svc::hal::gpio::Input>,
        PinDriver<'static, esp_idf_svc::hal::gpio::Gpio12, esp_idf_svc::hal::gpio::Input>,
        PinDriver<'static, esp_idf_svc::hal::gpio::Gpio14, esp_idf_svc::hal::gpio::Input>,
        PinDriver<'static, esp_idf_svc::hal::gpio::Gpio15, esp_idf_svc::hal::gpio::Input>,
        PinDriver<'static, esp_idf_svc::hal::gpio::Gpio16, esp_idf_svc::hal::gpio::Input>,
        PinDriver<'static, esp_idf_svc::hal::gpio::Gpio17, esp_idf_svc::hal::gpio::Input>,
        PinDriver<'static, esp_idf_svc::hal::gpio::Gpio18, esp_idf_svc::hal::gpio::Input>,
        PinDriver<'static, esp_idf_svc::hal::gpio::Gpio19, esp_idf_svc::hal::gpio::Input>,
    )>,
> = Mutex::new(None);

/// Incoming command from host.
#[derive(Debug, Deserialize)]
struct Request {
    id: String,
    cmd: String,
    args: serde_json::Value,
}

/// Outgoing response to host.
#[derive(Debug, Serialize)]
struct Response {
    id: String,
    ok: bool,
    result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;

    // UART0: TX=21, RX=20 (ESP32) — ESP32-C3 may use different pins; adjust for your board
    let config = UartConfig::new().baudrate(Hertz(115_200));
    let mut uart = UartDriver::new(
        peripherals.uart0,
        peripherals.pins.gpio21,
        peripherals.pins.gpio20,
        Option::<esp_idf_svc::hal::gpio::Gpio0>::None,
        Option::<esp_idf_svc::hal::gpio::Gpio1>::None,
        &config,
    )?;

    info!("Housaky ESP32 firmware ready on UART0 (115200)");

    let pins = peripherals.pins;

    {
        let mut guard = INPUT_PINS_STATE.lock().unwrap();
        *guard = Some((
            PinDriver::input(pins.gpio0).unwrap(),
            PinDriver::input(pins.gpio1).unwrap(),
            PinDriver::input(pins.gpio3).unwrap(),
            PinDriver::input(pins.gpio4).unwrap(),
            PinDriver::input(pins.gpio5).unwrap(),
            PinDriver::input(pins.gpio12).unwrap(),
            PinDriver::input(pins.gpio14).unwrap(),
            PinDriver::input(pins.gpio15).unwrap(),
            PinDriver::input(pins.gpio16).unwrap(),
            PinDriver::input(pins.gpio17).unwrap(),
            PinDriver::input(pins.gpio18).unwrap(),
            PinDriver::input(pins.gpio19).unwrap(),
        ));
    }

    let mut buf = [0u8; 512];
    let mut line = Vec::new();

    loop {
        match uart.read(&mut buf, 100) {
            Ok(0) => continue,
            Ok(n) => {
                for &b in &buf[..n] {
                    if b == b'\n' {
                        if !line.is_empty() {
                            if let Ok(line_str) = std::str::from_utf8(&line) {
                                if let Ok(resp) = handle_request(line_str, &peripherals) {
                                    let out = serde_json::to_string(&resp).unwrap_or_default();
                                    let _ = uart.write(format!("{}\n", out).as_bytes());
                                }
                            }
                            line.clear();
                        }
                    } else {
                        line.push(b);
                        if line.len() > 400 {
                            line.clear();
                        }
                    }
                }
            }
            Err(_) => {}
        }
    }
}

fn handle_request(
    line: &str,
    peripherals: &esp_idf_svc::hal::peripherals::Peripherals,
) -> anyhow::Result<Response> {
    let req: Request = serde_json::from_str(line.trim())?;
    let id = req.id.clone();

    let result = match req.cmd.as_str() {
        "capabilities" => {
            // Phase C: report GPIO pins and LED pin (matches Arduino protocol)
            let caps = serde_json::json!({
                "gpio": [0, 1, 2, 3, 4, 5, 12, 13, 14, 15, 16, 17, 18, 19],
                "led_pin": 2
            });
            Ok(caps.to_string())
        }
        "gpio_read" => {
            let pin_num = req.args.get("pin").and_then(|v| v.as_u64()).unwrap_or(0) as i32;
            let value = gpio_read(peripherals, pin_num)?;
            Ok(value.to_string())
        }
        "gpio_write" => {
            let pin_num = req.args.get("pin").and_then(|v| v.as_u64()).unwrap_or(0) as i32;
            let value = req.args.get("value").and_then(|v| v.as_u64()).unwrap_or(0);
            gpio_write(peripherals, pin_num, value)?;
            Ok("done".into())
        }
        _ => Err(anyhow::anyhow!("Unknown command: {}", req.cmd)),
    };

    match result {
        Ok(r) => Ok(Response {
            id,
            ok: true,
            result: r,
            error: None,
        }),
        Err(e) => Ok(Response {
            id,
            ok: false,
            result: String::new(),
            error: Some(e.to_string()),
        }),
    }
}

fn gpio_read(
    _peripherals: &esp_idf_svc::hal::peripherals::Peripherals,
    pin: i32,
) -> anyhow::Result<u8> {
    let guard = INPUT_PINS_STATE.lock().unwrap();
    let pins = guard
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Input pins not initialized"))?;

    let value = match pin {
        0 => pins.0.is_high()?,
        1 => pins.1.is_high()?,
        3 => pins.2.is_high()?,
        4 => pins.3.is_high()?,
        5 => pins.4.is_high()?,
        12 => pins.5.is_high()?,
        14 => pins.6.is_high()?,
        15 => pins.7.is_high()?,
        16 => pins.8.is_high()?,
        17 => pins.9.is_high()?,
        18 => pins.10.is_high()?,
        19 => pins.11.is_high()?,
        _ => anyhow::bail!("Pin {} not configured as input", pin),
    };

    Ok(if value { 1 } else { 0 })
}

fn gpio_write(
    peripherals: &esp_idf_svc::hal::peripherals::Peripherals,
    pin: i32,
    value: u64,
) -> anyhow::Result<()> {
    let pins = peripherals.pins;
    let level = value != 0;

    match pin {
        2 => {
            let mut out = PinDriver::output(pins.gpio2)?;
            out.set_level(esp_idf_svc::hal::gpio::Level::from(level))?;
        }
        13 => {
            let mut out = PinDriver::output(pins.gpio13)?;
            out.set_level(esp_idf_svc::hal::gpio::Level::from(level))?;
        }
        _ => anyhow::bail!("Pin {} not configured (add to gpio_write)", pin),
    }
    Ok(())
}
