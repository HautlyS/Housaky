use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotSwapState {
    pub current_binary_hash: String,
    pub current_generation: u64,
    pub socket_path: PathBuf,
    pub state_handoff_path: PathBuf,
}

pub struct HotSwapper {
    pub workspace_dir: PathBuf,
}

impl HotSwapper {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }

    pub fn prepare_state_handoff(&self, generation: u64) -> Result<PathBuf> {
        let handoff_dir = self
            .workspace_dir
            .join(".housaky")
            .join("hot_swap");
        std::fs::create_dir_all(&handoff_dir)?;
        let state_path = handoff_dir.join(format!("gen_{}.json", generation));
        Ok(state_path)
    }

    pub fn write_handoff_state(
        &self,
        state_path: &Path,
        state: &HotSwapState,
    ) -> Result<()> {
        let json = serde_json::to_string_pretty(state)?;
        std::fs::write(state_path, json)
            .context("Failed to write hot-swap handoff state")?;
        info!("Wrote hot-swap state to {:?}", state_path);
        Ok(())
    }

    pub fn read_handoff_state(&self, generation: u64) -> Result<Option<HotSwapState>> {
        let state_path = self
            .workspace_dir
            .join(".housaky")
            .join("hot_swap")
            .join(format!("gen_{}.json", generation));
        if !state_path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(&state_path)?;
        let state: HotSwapState = serde_json::from_str(&json)?;
        Ok(Some(state))
    }

    /// Spawn the new binary as a background process, then gracefully stop self.
    /// Uses exec() semantics on Unix: replaces the current process image.
    pub fn exec_into_new_binary(
        &self,
        new_binary: &Path,
        generation: u64,
        args: &[&str],
    ) -> Result<()> {
        if !new_binary.exists() {
            anyhow::bail!("New binary not found: {:?}", new_binary);
        }

        let state = HotSwapState {
            current_binary_hash: String::new(),
            current_generation: generation,
            socket_path: self
                .workspace_dir
                .join(".housaky")
                .join("hot_swap.sock"),
            state_handoff_path: self
                .workspace_dir
                .join(".housaky")
                .join("hot_swap")
                .join(format!("gen_{}.json", generation)),
        };

        let handoff_path = self.prepare_state_handoff(generation)?;
        self.write_handoff_state(&handoff_path, &state)?;

        info!(
            "Hot-swapping to generation {} binary: {:?}",
            generation, new_binary
        );

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let mut cmd = Command::new(new_binary);
            cmd.args(args);
            cmd.env("HOUSAKY_GENERATION", generation.to_string());
            cmd.env(
                "HOUSAKY_HANDOFF_STATE",
                handoff_path.to_string_lossy().to_string(),
            );
            let err = cmd.exec();
            anyhow::bail!("exec() failed: {}", err);
        }

        #[cfg(not(unix))]
        {
            warn!("Hot-swap via exec() is only supported on Unix. Spawning new process instead.");
            let mut cmd = Command::new(new_binary);
            cmd.args(args);
            cmd.env("HOUSAKY_GENERATION", generation.to_string());
            cmd.env(
                "HOUSAKY_HANDOFF_STATE",
                handoff_path.to_string_lossy().to_string(),
            );
            cmd.spawn().context("Failed to spawn new binary")?;
            Ok(())
        }
    }

    pub fn install_new_binary(
        &self,
        new_binary: &Path,
        install_path: &Path,
    ) -> Result<()> {
        let backup = install_path.with_extension("bak");
        if install_path.exists() {
            std::fs::copy(install_path, &backup)
                .context("Failed to backup current binary")?;
            info!("Backed up current binary to {:?}", backup);
        }

        std::fs::copy(new_binary, install_path)
            .context("Failed to install new binary")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(install_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(install_path, perms)?;
        }

        info!("Installed new binary to {:?}", install_path);
        Ok(())
    }

    pub fn rollback_binary(&self, install_path: &Path) -> Result<()> {
        let backup = install_path.with_extension("bak");
        if backup.exists() {
            std::fs::copy(&backup, install_path)
                .context("Failed to restore backup binary")?;
            info!("Rolled back binary at {:?} from backup", install_path);
            Ok(())
        } else {
            anyhow::bail!("No backup binary found at {:?}", backup)
        }
    }
}
