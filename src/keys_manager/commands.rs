use crate::config::Config;
use crate::keys_manager::manager::{KeysManager, ProviderPriority};
use anyhow::Result;
use clap::{Args, Subcommand};
use std::collections::HashMap;


#[derive(Debug, Subcommand, Clone)]
pub enum KeysManagerCommands {
    /// List providers/keys in keys.json store.
    List,

    /// Show aggregated key/provider stats.
    Stats,

    /// Launch interactive TUI.
    Tui,

    /// Add a provider based on a built-in template.
    AddProvider(KeysAddProviderArgs),

    /// Add a custom provider.
    AddCustomProvider(KeysAddCustomProviderArgs),

    /// Add a key to an existing provider.
    AddKey(KeysAddKeyArgs),

    /// Remove a key from an existing provider.
    RemoveKey(KeysRemoveKeyArgs),

    /// Rotate to next key for a provider (or all providers).
    Rotate(KeysRotateArgs),

    /// Enable a key by ID.
    EnableKey(KeysToggleKeyArgs),

    /// Disable a key by ID.
    DisableKey(KeysToggleKeyArgs),

    /// Remove a provider.
    RemoveProvider(KeysRemoveProviderArgs),

    /// Enable/disable a provider.
    SetProviderEnabled(KeysSetProviderEnabledArgs),

    /// Set provider priority (primary/secondary/tertiary/quaternary/disabled).
    SetPriority(KeysSetPriorityArgs),

    /// Set provider default model.
    SetDefaultModel(KeysSetDefaultModelArgs),

    /// Export keys.json (prints JSON or writes to file).
    Export(KeysExportArgs),

    /// Import keys.json from a JSON string or file.
    Import(KeysImportArgs),


    /// Sync the legacy config.toml api_key/default_provider/default_model into keys.json
    /// without overwriting an existing api_key in config.toml.
    SyncFromConfig,

    /// Print a config.toml snippet for task routing (per-subject routes).
    PrintRoutesExample,
}

#[derive(Debug, Args, Clone)]
pub struct KeysAddProviderArgs {
    pub name: String,
    #[arg(long, default_value = "openrouter")]
    pub template: String,
    #[arg(long = "key", short = 'k')]
    pub keys: Vec<String>,
    #[arg(long, default_value = "primary")]
    pub priority: String,
}

#[derive(Debug, Args, Clone)]
pub struct KeysAddCustomProviderArgs {
    pub name: String,
    #[arg(long)]
    pub base_url: String,
    #[arg(long, default_value = "bearer")]
    pub auth_method: String,
    #[arg(long = "key", short = 'k')]
    pub keys: Vec<String>,
    #[arg(long = "model", short = 'm')]
    pub models: Vec<String>,
}

#[derive(Debug, Args, Clone)]
pub struct KeysAddKeyArgs {
    pub provider: String,
    pub key: String,

    /// Optional human-friendly name for the key.
    #[arg(long)]
    pub name: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct KeysRemoveKeyArgs {
    pub provider: String,
    pub key_id: String,
}

#[derive(Debug, Args, Clone)]
pub struct KeysRotateArgs {
    /// Provider name. If omitted, rotates all providers.
    pub provider: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct KeysToggleKeyArgs {
    pub provider: String,
    pub key_id: String,
}

#[derive(Debug, Args, Clone)]
pub struct KeysRemoveProviderArgs {
    pub name: String,
}

#[derive(Debug, Args, Clone)]
pub struct KeysSetProviderEnabledArgs {
    pub provider: String,

    /// Enable the provider
    #[arg(long, conflicts_with = "disable")]
    pub enable: bool,

    /// Disable the provider
    #[arg(long, conflicts_with = "enable")]
    pub disable: bool,
}

#[derive(Debug, Args, Clone)]
pub struct KeysSetPriorityArgs {
    pub provider: String,
    pub priority: String,
}

#[derive(Debug, Args, Clone)]
pub struct KeysSetDefaultModelArgs {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Args, Clone)]
pub struct KeysExportArgs {
    /// Optional output file path. If omitted, prints JSON to stdout.
    #[arg(long)]
    pub path: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct KeysImportArgs {
    /// Input file path.
    #[arg(long)]
    pub path: Option<String>,

    /// Raw JSON string (if not using --path).
    #[arg(long, conflicts_with = "path")]
    pub json: Option<String>,
}


pub async fn handle_keys_manager_command(
    config: &mut Config,
    manager: &KeysManager,
    cmd: KeysManagerCommands,
) -> Result<()> {
    match cmd {
        KeysManagerCommands::List => {
            let _ = manager.load().await;
            let store = manager.store.read().await;
            if store.providers.is_empty() {
                println!("No providers configured in keys.json.");
                return Ok(());
            }
            for (name, p) in &store.providers {
                let enabled = p.keys.iter().filter(|k| k.enabled).count();
                println!(
                    "[{name}] keys={} enabled={} priority={} default_model={}",
                    p.keys.len(),
                    enabled,
                    p.priority,
                    p.default_model.as_deref().unwrap_or("<none>"),
                );
            }
            Ok(())
        }
        KeysManagerCommands::Stats => {
            let _ = manager.load().await;
            let stats = manager.get_stats().await;
            println!("Keys stats:");
            println!("  Providers: total={} enabled={} primary={}", stats.total_providers, stats.enabled_providers, stats.primary_providers);
            println!("  Keys:      total={} enabled={}", stats.total_keys, stats.enabled_keys);
            println!("  Health:    healthy={} rate_limited={}", stats.healthy_providers, stats.rate_limited_providers);
            println!("  Requests:  total={} ok={} failed={}", stats.total_requests, stats.successful_requests, stats.failed_requests);
            Ok(())
        }
        KeysManagerCommands::Tui => {
            // run_keys_tui runs blocking crossterm I/O (event::poll, enable_raw_mode, etc.).
            // Awaiting it directly inside the outer tokio runtime stalls the executor.
            // Spawn a dedicated OS thread with its own single-threaded tokio runtime.
            let (tx, rx) = std::sync::mpsc::channel::<anyhow::Result<()>>();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build keys-tui runtime");
                rt.block_on(async move {
                    let inner_manager = crate::keys_manager::manager::KeysManager::new();
                    let _ = inner_manager.load().await;
                    tx.send(crate::keys_manager::tui::run_keys_tui(&inner_manager).await).ok();
                });
            });
            rx.recv()
                .map_err(|e| anyhow::anyhow!("Keys TUI thread error: {e}"))??;
            Ok(())
        }
        KeysManagerCommands::AddProvider(args) => {
            let priority = ProviderPriority::from_str(&args.priority)
                .ok_or_else(|| anyhow::anyhow!("Invalid priority: {}", args.priority))?;
            manager
                .add_provider_with_template(&args.name, &args.template, args.keys, priority)
                .await?;
            println!("Added provider {}", args.name);
            Ok(())
        }
        KeysManagerCommands::AddCustomProvider(args) => {
            manager
                .add_custom_provider(
                    &args.name,
                    &args.base_url,
                    &args.auth_method,
                    args.keys,
                    args.models,
                    ProviderPriority::Primary,
                    HashMap::new(),
                )
                .await?;
            println!("Added custom provider {}", args.name);
            Ok(())
        }
        KeysManagerCommands::AddKey(args) => {
            manager
                .add_key(&args.provider, args.key, args.name.clone())
                .await?;
            println!("Added key to {}", args.provider);
            Ok(())
        }
        KeysManagerCommands::RemoveKey(args) => {
            manager.remove_key(&args.provider, &args.key_id).await?;
            println!("Removed key {} from {}", args.key_id, args.provider);
            Ok(())
        }
        KeysManagerCommands::Rotate(args) => {
            let _ = manager.load().await;
            if let Some(provider_name) = args.provider {
                match manager.rotate_key(&provider_name).await {
                    Ok(Some(key)) => {
                        let key_suffix = &key.key[key.key.len().saturating_sub(4)..];
                        println!("Rotated key for {}: ...{}", provider_name, key_suffix);
                        Ok(())
                    }
                    Ok(None) => {
                        println!("No keys available for provider: {}", provider_name);
                        Ok(())
                    }
                    Err(e) => {
                        println!("Error rotating key: {}", e);
                        Err(anyhow::anyhow!(e))
                    }
                }
            } else {
                match manager.rotate_all_keys().await {
                    Ok(results) => {
                        for (name, key) in results {
                            if let Some(k) = key {
                                let key_suffix = &k.key[k.key.len().saturating_sub(4)..];
                                println!("Rotated key for {}: ...{}", name, key_suffix);
                            }
                        }
                        Ok(())
                    }
                    Err(e) => {
                        println!("Error rotating keys: {}", e);
                        Err(anyhow::anyhow!(e))
                    }
                }
            }
        }
        KeysManagerCommands::EnableKey(args) => {
            manager
                .set_key_enabled(&args.provider, &args.key_id, true)
                .await?;
            println!("Enabled key {} in {}", args.key_id, args.provider);
            Ok(())
        }
        KeysManagerCommands::DisableKey(args) => {
            manager
                .set_key_enabled(&args.provider, &args.key_id, false)
                .await?;
            println!("Disabled key {} in {}", args.key_id, args.provider);
            Ok(())
        }
        KeysManagerCommands::RemoveProvider(args) => {
            manager.remove_provider(&args.name).await?;
            println!("Removed provider {}", args.name);
            Ok(())
        }
        KeysManagerCommands::SetProviderEnabled(args) => {
            let enabled = if args.enable {
                true
            } else if args.disable {
                false
            } else {
                anyhow::bail!("Pass either --enable or --disable");
            };
            manager
                .set_provider_enabled(&args.provider, enabled)
                .await?;
            println!("Updated provider {} enabled={}", args.provider, enabled);
            Ok(())
        }
        KeysManagerCommands::SetPriority(args) => {
            let priority = ProviderPriority::from_str(&args.priority)
                .ok_or_else(|| anyhow::anyhow!("Invalid priority: {}", args.priority))?;
            manager.set_provider_priority(&args.provider, priority).await?;
            println!("Updated priority");
            Ok(())
        }
        KeysManagerCommands::SetDefaultModel(args) => {
            manager.set_default_model(&args.provider, &args.model).await?;
            println!("Updated default model");
            Ok(())
        }
        KeysManagerCommands::Export(args) => {
            let _ = manager.load().await;
            let json = manager.export().await?;
            if let Some(path) = args.path {
                std::fs::write(&path, &json)?;
                println!("Exported keys.json to {path}");
                Ok(())
            } else {
                println!("{json}");
                Ok(())
            }
        }
        KeysManagerCommands::Import(args) => {
            let input = if let Some(path) = args.path {
                std::fs::read_to_string(&path)?
            } else if let Some(json) = args.json {
                json
            } else {
                anyhow::bail!("Provide either --path or --json");
            };
            manager.import(&input).await?;
            println!("Imported keys.json store");
            Ok(())
        }

        KeysManagerCommands::SyncFromConfig => {
            let _ = manager.load().await;

            // Preserve existing config.api_key by NOT overwriting it.
            // If config.api_key exists and keys store has no providers yet, create one.
            let api_key = config.api_key.clone().unwrap_or_default();
            let provider = config
                .default_provider
                .clone()
                .unwrap_or_else(|| "openrouter".to_string());

            if !api_key.trim().is_empty() {
                // Only inject if provider does not exist or has no keys.
                let store = manager.store.write().await;
                let needs_key = store
                    .providers
                    .get(&provider)
                    .map(|p| p.keys.is_empty())
                    .unwrap_or(true);
                drop(store);

                if needs_key {
                    // Use template if known.
                    let template = if manager.provider_templates.contains_key(provider.as_str()) {
                        provider.as_str()
                    } else {
                        "custom"
                    };
                    let _ = manager
                        .add_provider_with_template(
                            &provider,
                            template,
                            vec![api_key.clone()],
                            ProviderPriority::Primary,
                        )
                        .await;
                }
            }

            // Default model, if present.
            if let Some(model) = config.default_model.clone() {
                let _ = manager.set_default_model(&provider, &model).await;
            }

            manager.save().await?;
            println!("Synced from config.toml into keys.json (did not modify config.toml api_key)." );
            Ok(())
        }
        KeysManagerCommands::PrintRoutesExample => {
            println!(
                "# Example per-subject routing (use with RouterProvider: hint:<subject>)\n\
[[model_routes]]\nhint = \"code\"\nprovider = \"openrouter\"\nmodel = \"anthropic/claude-sonnet-4-20250514\"\n\n\
[[model_routes]]\nhint = \"review\"\nprovider = \"anthropic\"\nmodel = \"claude-opus-4-20250514\"\n\n\
[[model_routes]]\nhint = \"debug\"\nprovider = \"openai\"\nmodel = \"gpt-4o\"\n"
            );
            Ok(())
        }
    }
}
