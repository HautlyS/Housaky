import sys

with open('src/main.rs', 'r') as f:
    content = f.read()

old_fn = """async fn start_full_system(config: Config, verbose: bool) -> Result<()> {"""
new_fn = """async fn start_full_system(config: Config, provider: Option<String>, model: Option<String>, verbose: bool) -> Result<()> {"""

content = content.replace(old_fn, new_fn)

old_call = """    let result = housaky::housaky::heartbeat::run_agi_with_tui(
        config.clone(),
        None, // No initial message
        None, // Use default provider
        None, // Use default model
        verbose,
    )
    .await;"""

new_call = """    let result = housaky::housaky::heartbeat::run_agi_with_tui(
        config.clone(),
        None, // No initial message
        provider,
        model,
        verbose,
    )
    .await;"""

content = content.replace(old_call, new_call)

# Update calls to start_full_system
content = content.replace("start_full_system(config, false).await", "start_full_system(config, None, None, false).await")
content = content.replace("start_full_system(config, provider, model, false).await", "start_full_system(config, provider, model, false).await") # actually this one is what I want for Tui command

with open('src/main.rs', 'w') as f:
    f.write(content)
