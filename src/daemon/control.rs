use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

/// Kill any running `housaky daemon` process and wait for it to exit.
pub fn stop() {
    let pids = find_daemon_pids();
    if pids.is_empty() {
        println!("ℹ️  No running Housaky daemon found.");
        return;
    }
    for pid in &pids {
        println!("🛑 Stopping daemon (PID {pid})...");
        let _ = Command::new("kill").arg(pid).status();
    }
    // Wait up to 5 s for processes to exit
    for _ in 0..25 {
        sleep(Duration::from_millis(200));
        if find_daemon_pids().is_empty() {
            break;
        }
    }
    // Force-kill anything still alive
    let remaining = find_daemon_pids();
    for pid in &remaining {
        let _ = Command::new("kill").args(["-9", pid]).status();
    }
    // Also kill any orphaned ngrok tunnel
    let _ = Command::new("pkill").args(["-f", "ngrok"]).status();
    println!("✅ Daemon stopped.");
}

/// Print whether the daemon is running.
pub fn status() {
    let pids = find_daemon_pids();
    if pids.is_empty() {
        println!("💤 Housaky daemon is NOT running.");
    } else {
        println!("✅ Housaky daemon is running (PID {}).", pids.join(", "));
        // Try the local health endpoint
        let port = 8080u16;
        let reachable = std::net::TcpStream::connect_timeout(
            &format!("127.0.0.1:{port}").parse().unwrap(),
            Duration::from_secs(2),
        )
        .is_ok();
        if reachable {
            println!("   Gateway: reachable on http://127.0.0.1:{port}");
            println!("   Health:  http://127.0.0.1:{port}/health");
        } else {
            println!("   Gateway: not yet reachable on port {port}");
        }
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn find_daemon_pids() -> Vec<String> {
    let my_pid = std::process::id().to_string();
    let out = Command::new("pgrep")
        .args(["-f", "housaky daemon"])
        .output()
        .unwrap_or_else(|_| std::process::Output {
            status: std::process::ExitStatus::default(),
            stdout: vec![],
            stderr: vec![],
        });
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .filter(|pid| *pid != my_pid)
        .map(str::to_owned)
        .collect()
}
