use anyhow::Result;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::process::Command;
use tracing::info;

pub struct DashboardStatus {
    pub installed: bool,
    pub running: bool,
    pub port: Option<u16>,
    pub url: Option<String>,
}

fn get_dashboard_paths() -> (PathBuf, PathBuf) {
    // First check for built dashboard in the project directory (development)
    let project_dashboard = std::env::current_dir()
        .map(|d| d.join("dashboard").join("dist"))
        .unwrap_or_default();
    
    if project_dashboard.exists() && project_dashboard.join("index.html").exists() {
        let dashboard_dir = project_dashboard.parent().unwrap().to_path_buf();
        return (dashboard_dir, project_dashboard);
    }
    
    // Then check in the config directory (installed)
    let config_dir = directories::ProjectDirs::from("com", "housaky", "housaky")
        .map(|pd| pd.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    
    let dashboard_dir = config_dir.join("dashboard");
    let dist_dir = dashboard_dir.join("dist");
    
    (dashboard_dir, dist_dir)
}

pub fn check_dashboard_installed() -> bool {
    let (_, dist_dir) = get_dashboard_paths();
    dist_dir.exists() && dist_dir.join("index.html").exists()
}

pub fn check_dashboard_running(port: u16) -> bool {
    use std::net::TcpStream;
    TcpStream::connect(("127.0.0.1", port)).is_ok()
}

pub fn get_dashboard_status(port: u16) -> DashboardStatus {
    let installed = check_dashboard_installed();
    let running = if installed { check_dashboard_running(port) } else { false };
    
    DashboardStatus {
        installed,
        running,
        port: if running { Some(port) } else { None },
        url: if running { Some(format!("http://localhost:{}", port)) } else { None },
    }
}

pub fn print_status(port: u16) {
    let status = get_dashboard_status(port);
    
    println!("📊 Housaky Dashboard Status");
    println!();
    println!("  Installed:  {}", if status.installed { "✅ Yes" } else { "❌ No" });
    println!("  Running:    {}", if status.running { "✅ Yes" } else { "❌ No" });
    
    if let Some(url) = &status.url {
        println!();
        println!("🔗 Dashboard URL: {}", url);
    }
    
    if !status.installed {
        println!();
        println!("💡 To install the dashboard:");
        println!("   1. Build from source: cd dashboard && pnpm install && pnpm tauri build");
        println!("   2. Or download from: https://github.com/HautlyS/Housaky/releases");
    }
    
    if status.installed && !status.running {
        println!();
        println!("💡 To start the dashboard:");
        println!("   housaky dashboard --start");
    }
}

pub async fn start_dashboard_server(host: &str, port: u16, open: bool) -> Result<()> {
    let (_, dist_dir) = get_dashboard_paths();
    
    if !dist_dir.exists() {
        anyhow::bail!(
            "Dashboard not installed. Build it with: cd dashboard && pnpm install && pnpm build\n\
             Or download from: https://github.com/HautlyS/Housaky/releases"
        );
    }
    
    let bind_addr: IpAddr = if host == "0.0.0.0" || host == "network" {
        IpAddr::V4(Ipv4Addr::UNSPECIFIED)
    } else {
        host.parse().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
    };
    
    let addr = SocketAddr::new(bind_addr, port);
    let display_ip = if addr.ip().is_unspecified() { "localhost" } else { &host };
    let url = format!("http://{}:{}", display_ip, port);
    
    println!("🚀 Starting Housaky Dashboard...");
    println!();
    println!("   Address: {}", addr);
    println!("   URL:     {}", url);
    
    if addr.ip().is_unspecified() {
        println!();
        println!("⚠️  Dashboard is exposed to network (0.0.0.0)");
        println!("   Access from other devices using your local IP");
    }
    
    if open {
        println!();
        println!("🌐 Opening dashboard in browser...");
        if let Err(e) = open_url(&url) {
            println!("   Could not open browser: {}", e);
        }
    }
    
    println!();
    println!("Press Ctrl+C to stop the server");
    println!();
    
    let dist = dist_dir.clone();
    
    tokio::spawn(async move {
        use tokio::net::TcpListener;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        
        let listener = match TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to bind to {}: {}", addr, e);
                return;
            }
        };
        
        info!("Dashboard server listening on {}", addr);
        
        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(conn) => conn,
                Err(_) => continue,
            };
            
            let dist_clone = dist.clone();
            
            tokio::spawn(async move {
                let mut buffer = [0; 4096];
                let n = match socket.read(&mut buffer).await {
                    Ok(n) if n > 0 => n,
                    _ => return,
                };
                
                let request = String::from_utf8_lossy(&buffer[..n]);
                let path = extract_path(&request);
                
                let (content, content_type) = serve_file(&dist_clone, &path).await;
                
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
                    content_type,
                    content.len()
                );
                
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.write_all(&content).await;
            });
        }
    });
    
    tokio::signal::ctrl_c().await?;
    println!("\n\nDashboard server stopped.");
    
    Ok(())
}

fn extract_path(request: &str) -> String {
    let first_line = request.lines().next().unwrap_or("GET / HTTP/1.1");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].trim_start_matches('/').to_string()
    } else {
        String::new()
    }
}

async fn serve_file(dist_dir: &PathBuf, path: &str) -> (Vec<u8>, &'static str) {
    let file_path = if path.is_empty() || path == "index.html" {
        dist_dir.join("index.html")
    } else {
        dist_dir.join(path)
    };
    
    if let Ok(content) = tokio::fs::read(&file_path).await {
        let content_type = match file_path.extension().and_then(|e| e.to_str()) {
            Some("html") => "text/html",
            Some("css") => "text/css",
            Some("js") => "application/javascript",
            Some("json") => "application/json",
            Some("png") => "image/png",
            Some("jpg" | "jpeg") => "image/jpeg",
            Some("svg") => "image/svg+xml",
            Some("ico") => "image/x-icon",
            Some("woff" | "woff2") => "font/woff2",
            _ => "application/octet-stream",
        };
        (content, content_type)
    } else {
        let fallback = dist_dir.join("index.html");
        match tokio::fs::read(&fallback).await {
            Ok(content) => (content, "text/html"),
            Err(_) => (b"<html><body><h1>Dashboard not found</h1></body></html>".to_vec(), "text/html"),
        }
    }
}

fn open_url(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn()?;
    }
    
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(url).spawn()?;
    }
    
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()?;
    }
    
    Ok(())
}

pub fn launch_desktop_app() -> Result<()> {
    let (dashboard_dir, _) = get_dashboard_paths();
    
    #[cfg(target_os = "macos")]
    {
        let app_path = dashboard_dir.join("Housaky Dashboard.app");
        if app_path.exists() {
            Command::new("open").arg(&app_path).spawn()?;
            println!("✅ Launched Housaky Dashboard app");
            return Ok(());
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        let appimage_path = dashboard_dir.join("housaky-dashboard");
        if appimage_path.exists() {
            Command::new(&appimage_path).spawn()?;
            println!("✅ Launched Housaky Dashboard app");
            return Ok(());
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        let exe_path = dashboard_dir.join("Housaky Dashboard.exe");
        if exe_path.exists() {
            Command::new(&exe_path).spawn()?;
            println!("✅ Launched Housaky Dashboard app");
            return Ok(());
        }
    }
    
    println!("❌ Dashboard desktop app not found");
    println!("   Build it with: cd dashboard && pnpm tauri build");
    println!("   Or use --start to start the web version");
    
    Ok(())
}
