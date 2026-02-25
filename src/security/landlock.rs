//! Landlock sandbox (Linux kernel 5.13+ LSM)
//!
//! Landlock provides unprivileged sandboxing through the Linux kernel.
//! This module uses the pure-Rust `landlock` crate for filesystem access control.

#[cfg(all(feature = "sandbox-landlock", target_os = "linux"))]
use landlock::{path_beneath_rules, Access, AccessFs, Ruleset, RulesetAttr, RulesetCreatedAttr, ABI};

use crate::security::traits::Sandbox;
use std::path::Path;

/// Landlock sandbox backend for Linux
#[cfg(all(feature = "sandbox-landlock", target_os = "linux"))]
#[derive(Debug)]
pub struct LandlockSandbox {
    workspace_dir: Option<std::path::PathBuf>,
}

#[cfg(all(feature = "sandbox-landlock", target_os = "linux"))]
impl LandlockSandbox {
    /// Create a new Landlock sandbox with the given workspace directory
    pub fn new() -> std::io::Result<Self> {
        Self::with_workspace(None)
    }

    /// Create a Landlock sandbox with a specific workspace directory
    pub fn with_workspace(workspace_dir: Option<std::path::PathBuf>) -> std::io::Result<Self> {
        // Test if Landlock is available by trying to create a minimal ruleset
        let test_ruleset = Ruleset::default().handle_access(AccessFs::from_all(ABI::V6));

        match test_ruleset.and_then(|r| r.create()) {
            Ok(_) => Ok(Self { workspace_dir }),
            Err(e) => {
                tracing::debug!("Landlock not available: {}", e);
                Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "Landlock not available",
                ))
            }
        }
    }

    /// Probe if Landlock is available (for auto-detection)
    pub fn probe() -> std::io::Result<Self> {
        Self::new()
    }

    /// Apply Landlock restrictions to the current process
    fn apply_restrictions(&self) -> std::io::Result<()> {
        let abi = ABI::V6;
        let ruleset = Ruleset::default()
            .handle_access(AccessFs::from_all(abi))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let created = ruleset
            .create()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let mut created = created;

        // Allow workspace directory (read/write)
        if let Some(ref workspace) = self.workspace_dir {
            if workspace.exists() {
                created = created
                    .add_rules(path_beneath_rules([workspace], AccessFs::from_all(abi)))
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            }
        }

        // Allow /tmp for general operations
        created = created
            .add_rules(path_beneath_rules([Path::new("/tmp")], AccessFs::from_all(abi)))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        // Allow /usr and /bin for executing commands (read only)
        created = created
            .add_rules(path_beneath_rules(
                [Path::new("/usr"), Path::new("/bin")],
                AccessFs::from_read(abi),
            ))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        // Apply to self
        created
            .restrict_self()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        tracing::debug!("Landlock restrictions applied successfully");
        Ok(())
    }
}

#[cfg(all(feature = "sandbox-landlock", target_os = "linux"))]
impl Sandbox for LandlockSandbox {
    fn wrap_command(&self, _cmd: &mut std::process::Command) -> std::io::Result<()> {
        // Apply Landlock restrictions before executing the command
        // Note: This affects the current process, not the child process
        // Child processes inherit the Landlock restrictions
        self.apply_restrictions()
    }

    fn is_available(&self) -> bool {
        // Try to create a minimal ruleset to verify availability
        Ruleset::default()
            .handle_access(AccessFs::from_read(ABI::V6))
            .and_then(|r| r.create())
            .is_ok()
    }

    fn name(&self) -> &str {
        "landlock"
    }

    fn description(&self) -> &str {
        "Linux kernel LSM sandboxing (filesystem access control)"
    }
}

// Stub implementations for non-Linux or when feature is disabled
#[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
pub struct LandlockSandbox;

#[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
impl LandlockSandbox {
    pub fn new() -> std::io::Result<Self> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Landlock is only supported on Linux with the sandbox-landlock feature",
        ))
    }

    pub fn with_workspace(_workspace_dir: Option<std::path::PathBuf>) -> std::io::Result<Self> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Landlock is only supported on Linux",
        ))
    }

    pub fn probe() -> std::io::Result<Self> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Landlock is only supported on Linux",
        ))
    }
}

#[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
impl Sandbox for LandlockSandbox {
    fn wrap_command(&self, _cmd: &mut std::process::Command) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Landlock is only supported on Linux",
        ))
    }

    fn is_available(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "landlock"
    }

    fn description(&self) -> &str {
        "Linux kernel LSM sandboxing (not available on this platform)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(feature = "sandbox-landlock", target_os = "linux"))]
    #[test]
    fn landlock_sandbox_name() {
        if let Ok(sandbox) = LandlockSandbox::new() {
            assert_eq!(sandbox.name(), "landlock");
        }
    }

    #[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
    #[test]
    fn landlock_not_available_on_non_linux() {
        let sandbox = LandlockSandbox;
        assert!(!sandbox.is_available());
        assert_eq!(sandbox.name(), "landlock");
    }

    #[test]
    fn landlock_with_none_workspace() {
        // Should work even without a workspace directory
        let result = LandlockSandbox::with_workspace(None);
        // Result depends on platform and feature flag
        match result {
            Ok(sandbox) => assert!(sandbox.is_available()),
            Err(_) => assert!(!cfg!(all(
                feature = "sandbox-landlock",
                target_os = "linux"
            ))),
        }
    }
}
