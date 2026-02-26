use super::{kill_shared, new_shared_process, SharedProcess, Tunnel, TunnelProcess};
use anyhow::{bail, Result};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;

/// ngrok Tunnel — wraps the `ngrok` binary.
///
/// Requires `ngrok` installed. Optionally set a custom domain
/// (requires ngrok paid plan).
pub struct NgrokTunnel {
    auth_token: String,
    domain: Option<String>,
    proc: SharedProcess,
}

impl NgrokTunnel {
    pub fn new(auth_token: String, domain: Option<String>) -> Self {
        Self {
            auth_token,
            domain,
            proc: new_shared_process(),
        }
    }
}

#[async_trait::async_trait]
impl Tunnel for NgrokTunnel {
    fn name(&self) -> &str {
        "ngrok"
    }

    async fn start(&self, _local_host: &str, local_port: u16) -> Result<String> {
        // Set auth token
        Command::new("ngrok")
            .args(["config", "add-authtoken", &self.auth_token])
            .output()
            .await?;

        // Write ngrok logs to a temp file — NOT stdout pipe.
        // If ngrok writes to a piped stdout and the reader is dropped after
        // we find the URL, ngrok receives SIGPIPE and exits. Using a log file
        // keeps the process alive for the full server lifetime.
        let log_path = std::env::temp_dir().join("housaky_ngrok.log");
        // Truncate any previous log so we read fresh output
        std::fs::write(&log_path, b"").ok();

        // Build command: ngrok http <port> [--url <domain>]
        let mut args = vec!["http".to_string(), local_port.to_string()];
        if let Some(ref domain) = self.domain {
            args.push("--url".into());
            args.push(domain.clone());
        }
        args.push("--log".into());
        args.push(log_path.to_string_lossy().into_owned());
        args.push("--log-format".into());
        args.push("logfmt".into());

        let child = Command::new("ngrok")
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()?;

        // Poll the log file for the tunnel URL
        let mut public_url = String::new();
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(20);
        while tokio::time::Instant::now() < deadline {
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            let contents = tokio::fs::read_to_string(&log_path).await.unwrap_or_default();
            for line in contents.lines() {
                tracing::debug!("ngrok: {line}");
                if line.contains("ERR_NGROK") || line.contains("authentication failed") {
                    bail!("ngrok error: {line}");
                }
                // logfmt: url=https://...
                if let Some(idx) = line.find("url=https://") {
                    let url_part = &line[idx + 4..];
                    let end = url_part
                        .find(|c: char| c.is_whitespace())
                        .unwrap_or(url_part.len());
                    public_url = url_part[..end].to_string();
                    break;
                }
            }
            if !public_url.is_empty() {
                break;
            }
        }

        if public_url.is_empty() {
            bail!("ngrok did not produce a public URL within 20s. Check auth token and domain.");
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

    #[test]
    fn constructor_stores_domain() {
        let tunnel = NgrokTunnel::new("ngrok-token".into(), Some("my.ngrok.app".into()));
        assert_eq!(tunnel.domain.as_deref(), Some("my.ngrok.app"));
    }

    #[test]
    fn public_url_is_none_before_start() {
        let tunnel = NgrokTunnel::new("ngrok-token".into(), None);
        assert!(tunnel.public_url().is_none());
    }

    #[tokio::test]
    async fn stop_without_started_process_is_ok() {
        let tunnel = NgrokTunnel::new("ngrok-token".into(), None);
        let result = tunnel.stop().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn health_check_is_false_before_start() {
        let tunnel = NgrokTunnel::new("ngrok-token".into(), None);
        assert!(!tunnel.health_check().await);
    }
}
