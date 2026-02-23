pub mod cloudflare;
pub mod docker;
pub mod native;
pub mod traits;
pub mod wasm;

pub use cloudflare::CloudflareRuntime;
pub use docker::DockerRuntime;
pub use native::NativeRuntime;
pub use traits::RuntimeAdapter;
pub use wasm::WasmRuntime;

use crate::config::RuntimeConfig;

/// Factory: create the right runtime from config
pub fn create_runtime(config: &RuntimeConfig) -> anyhow::Result<Box<dyn RuntimeAdapter>> {
    match config.kind.as_str() {
        "native" => Ok(Box::new(NativeRuntime::new())),
        "docker" => Ok(Box::new(DockerRuntime::new(config.docker.clone()))),
        "wasm" => Ok(Box::new(WasmRuntime::new(config.wasm.clone()))),
        "cloudflare" => Ok(Box::new(CloudflareRuntime::new(config.cloudflare.clone()))),
        other if other.trim().is_empty() => {
            anyhow::bail!("runtime.kind cannot be empty. Supported values: native, docker, wasm, cloudflare")
        }
        other => anyhow::bail!("Unknown runtime kind '{other}'. Supported values: native, docker, wasm, cloudflare"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_native() {
        let cfg = RuntimeConfig {
            kind: "native".into(),
            ..RuntimeConfig::default()
        };
        let rt = create_runtime(&cfg).unwrap();
        assert_eq!(rt.name(), "native");
        assert!(rt.has_shell_access());
    }

    #[test]
    fn factory_docker() {
        let cfg = RuntimeConfig {
            kind: "docker".into(),
            ..RuntimeConfig::default()
        };
        let rt = create_runtime(&cfg).unwrap();
        assert_eq!(rt.name(), "docker");
        assert!(rt.has_shell_access());
    }

    #[test]
    fn factory_wasm() {
        let cfg = RuntimeConfig {
            kind: "wasm".into(),
            ..RuntimeConfig::default()
        };
        let rt = create_runtime(&cfg).unwrap();
        assert_eq!(rt.name(), "wasm");
        assert!(!rt.has_shell_access());
    }

    #[test]
    fn factory_cloudflare() {
        let mut cfg = RuntimeConfig::default();
        cfg.kind = "cloudflare".into();
        cfg.cloudflare.account_id = "test-account".into();
        cfg.cloudflare.api_token = "test-token".into();
        let rt = create_runtime(&cfg).unwrap();
        assert_eq!(rt.name(), "cloudflare");
        assert!(!rt.has_shell_access());
    }

    #[test]
    fn factory_unknown_errors() {
        let cfg = RuntimeConfig {
            kind: "unknown".into(),
            ..RuntimeConfig::default()
        };
        match create_runtime(&cfg) {
            Err(err) => assert!(err.to_string().contains("Unknown runtime kind")),
            Ok(_) => panic!("unknown runtime should error"),
        }
    }

    #[test]
    fn factory_empty_errors() {
        let cfg = RuntimeConfig {
            kind: String::new(),
            ..RuntimeConfig::default()
        };
        match create_runtime(&cfg) {
            Err(err) => assert!(err.to_string().contains("cannot be empty")),
            Ok(_) => panic!("empty runtime should error"),
        }
    }
}
