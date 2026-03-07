use crate::config::Config;

pub struct HelpSystem;

impl HelpSystem {
    pub fn show_help(topic: Option<&str>) {
        let topic = topic.unwrap_or("all");

        match topic {
            "tips" | "t" => Self::show_tips(),
            "commands" | "cmd" => Self::show_commands(),
            "keys" | "k" => Self::show_keys_help(),
            "skills" | "s" => Self::show_skills_help(),
            "mcp" | "m" => Self::show_mcp_help(),
            "agi" | "a" => Self::show_agi_help(),
            "quantum" | "q" => Self::show_quantum_help(),
            "all" | "h" | "help" => Self::show_all_help(),
            _ => Self::show_all_help(),
        }
    }

    fn show_tips() {
        println!(
            r#"
╔══════════════════════════════════════════════════════════════════════════════╗
║                           HOUSAKY TIPS & TRICKS                           ║
╠══════════════════════════════════════════════════════════════════════════════╣
║                                                                              ║
║  🚀 QUICK START                                                              ║
║     housaky init                    # Initialize workspace                   ║
║     housaky chat                    # Start chatting                         ║
║     housaky tui                     # Open unified TUI                     ║
║                                                                              ║
║  ⌨️  KEYBOARD SHORTCUTS (in TUI)                                            ║
║     Ctrl+C          Exit                                                       ║
║     Ctrl+K          Command palette                                           ║
║     Ctrl+S          Save/state                                                ║
║     Ctrl+H          Help                                                      ║
║     Tab             Autocomplete                                              ║
║     Esc             Go back/close                                             ║
║                                                                              ║
║  💡 PRO TIPS                                                                 ║
║     • Use `housaky chat -m "prompt"` for one-shot without TUI               ║
║     • Set default model: housaky config set default_model <model>           ║
║     • Enable skills with: housaky skills get <name> --enable                 ║
║     • Use lucid memory for best results: housaky init --memory lucid         ║
║     • Subagents auto-share context through inner monologue                   ║
║                                                                              ║
║  🔗 A2A COMMUNICATION                                                         ║
║     housaky a2a ping              # Test connection                         ║
║     housaky a2a sync              # Sync state with peer                    ║
║     housaky a2a learn <cat>       # Share learning                          ║
║                                                                              ║
╚══════════════════════════════════════════════════════════════════════════════╝
"#
        );
    }

    fn show_commands() {
        println!(
            r#"
╔══════════════════════════════════════════════════════════════════════════════╗
║                            HOUSAKY COMMANDS                                 ║
╠══════════════════════════════════════════════════════════════════════════════╣
║                                                                              ║
║  CORE                                                                          ║
║    housaky chat [-m <msg>]        Chat with AI                              ║
║    housaky tui [name]              Open unified TUI                          ║
║    housaky init [--interactive]    Initialize workspace                      ║
║    housaky status                   Show system status                       ║
║    housaky doctor                   Run diagnostics                          ║
║    housaky help [topic]             Show help                               ║
║                                                                              ║
║  KEYS & PROVIDERS                                                            ║
║    housaky keys list                List API keys                            ║
║    housaky keys add <name>          Add new key                            ║
║    housaky keys rotate <name>       Rotate key                             ║
║    housaky models list              List available models                    ║
║                                                                              ║
║  SKILLS & MCP                                                                ║
║    housaky skills list              List installed skills                   ║
║    housaky skills get <name>        Get/install skill                       ║
║    housaky skills ui                Skills marketplace TUI                   ║
║    housaky mcp list                 List available MCPs                      ║
║    housaky mcp install <name>       Install MCP server                       ║
║                                                                              ║
║  AGI & SELF-IMPROVEMENT                                                      ║
║    housaky goals                    Manage goals                             ║
║    housaky thoughts [n]             Show inner monologue                    ║
║    housaky improve                  Trigger self-improvement                  ║
║    housaky collective               Global collective intelligence          ║
║                                                                              ║
║  A2A & COLLABORATION                                                          ║
║    housaky a2a ping                 Ping peer instance                       ║
║    housaky a2a sync                 Sync state                              ║
║    housaky a2a delegate <task>      Delegate to peer                        ║
║    housaky migrate openclaw         Import from OpenClaw                    ║
║                                                                              ║
║  CONFIGURATION                                                                ║
║    housaky config edit              Edit config                              ║
║    housaky config set <key> <val>   Set config value                        ║
║    housaky config show              Show config                              ║
║                                                                              ║
║  RUNTIME                                                                      ║
║    housaky daemon start              Start daemon                          ║
║    housaky gateway start            Start HTTP gateway                       ║
║    housaky channel start            Start channels                          ║
║    housaky heartbeat                Trigger heartbeat                        ║
║                                                                              ║
╚══════════════════════════════════════════════════════════════════════════════╝
"#
        );
    }

    fn show_keys_help() {
        println!(
            r#"
╔══════════════════════════════════════════════════════════════════════════════╗
║                              KEYS MANAGEMENT                                 ║
╠══════════════════════════════════════════════════════════════════════════════╣
║                                                                              ║
║  COMMANDS                                                                    ║
║    housaky keys list                  List all keys                          ║
║    housaky keys add <name>            Add new API key                        ║
║    housaky keys remove <name>         Remove key                            ║
║    housaky keys rotate <name>         Rotate key                             ║
║    housaky keys test <name>           Test key                               ║
║    housaky keys export                Export keys (encrypted)                ║
║                                                                              ║
║  SUBAGENTS                                                                   ║
║    housaky keys subagent add <name>   Add subagent key                      ║
║    housaky keys subagent list         List subagent keys                    ║
║                                                                              ║
║  PROVIDERS                                                                   ║
║    OpenRouter:   housaky models list --provider openrouter                  ║
║    Anthropic:    housaky models list --provider anthropic                    ║
║    OpenAI:       housaky models list --provider openai                       ║
║                                                                              ║
╚══════════════════════════════════════════════════════════════════════════════╝
"#
        );
    }

    fn show_skills_help() {
        println!(
            r#"
╔══════════════════════════════════════════════════════════════════════════════╗
║                              SKILLS SYSTEM                                   ║
╠══════════════════════════════════════════════════════════════════════════════╣
║                                                                              ║
║  COMMANDS                                                                    ║
║    housaky skills list              List installed skills                    ║
║    housaky skills get <name>        Get/install skill from marketplace     ║
║    housaky skills ui                Open skills marketplace TUI             ║
║    housaky skills install <url>     Install from GitHub URL                 ║
║    housaky skills remove <name>     Remove skill                            ║
║    housaky skills convert <path>   Convert SKILL.md to SKILL.toml          ║
║                                                                              ║
║  MARKETPLACE SOURCES                                                         ║
║    • Claude Official Plugins    - Integrated with marketplace                ║
║    • OpenClaw Vendored         - Community skills                           ║
║    • GitHub URLs               - Install from any repo                      ║
║                                                                              ║
║  CREATING SKILLS                                                             ║
║    mkdir -p ~/.housaky/workspace/skills/my-skill                          ║
║    echo '# My Skill' > ~/.housaky/workspace/skills/my-skill/SKILL.md       ║
║                                                                              ║
╚══════════════════════════════════════════════════════════════════════════════╝
"#
        );
    }

    fn show_mcp_help() {
        println!(
            r#"
╔══════════════════════════════════════════════════════════════════════════════╗
║                    MCP (Model Context Protocol)                              ║
╠══════════════════════════════════════════════════════════════════════════════╣
║                                                                              ║
║  COMMANDS                                                                    ║
║    housaky mcp list                 List available MCP servers               ║
║    housaky mcp installed             List installed MCP servers              ║
║    housaky mcp install <name>       Install MCP server                      ║
║    housaky mcp uninstall <name>     Uninstall MCP server                    ║
║    housaky mcp enable <name>        Enable MCP server                       ║
║    housaky mcp disable <name>       Disable MCP server                       ║
║                                                                              ║
║  POPULAR MCP SERVERS                                                         ║
║    • filesystem                    - File system operations                  ║
║    • brave-search                  - Web search                             ║
║    • slack                         - Slack integration                      ║
║    • github                        - GitHub integration                     ║
║    • postgres                      - PostgreSQL database                     ║
║                                                                              ║
╚══════════════════════════════════════════════════════════════════════════════╝
"#
        );
    }

    fn show_agi_help() {
        println!(
            r#"
╔══════════════════════════════════════════════════════════════════════════════╗
║                         AGI & SELF-IMPROVEMENT                              ║
╠══════════════════════════════════════════════════════════════════════════════╣
║                                                                              ║
║  COMMANDS                                                                    ║
║    housaky goals                    Manage goals                             ║
║    housaky thoughts [n]             Show inner monologue (thoughts)         ║
║    housaky improve                  Trigger self-improvement cycle          ║
║    housaky selfmod                  Self-modification config                ║
║    housaky collective               Global collective intelligence           ║
║                                                                              ║
║  SUBAGENTS                                                                   ║
║    Housaky has 7 subagents that share consciousness:                       ║
║    • kowalski-code        - Code specialist                                ║
║    • kowalski-web         - Web researcher                                  ║
║    • kowalski-academic    - Academic analyst                               ║
║    • kowalski-data        - Data processor                                 ║
║    • kowalski-creative    - Creative synthesizer                           ║
║    • kowalski-reasoning   - Reasoning engine                               ║
║    • kowalski-federation - Coordinator (aware of all)                     ║
║                                                                              ║
║  MEMORY                                                                       ║
║    • Lucid (default)     - ACT-R spreading activation, shared with OpenClaw ║
║    • SQLite              - Vector embeddings + keyword search               ║
║    • Markdown            - Human-readable flat files                         ║
║    • None                - No persistence                                   ║
║                                                                              ║
╚══════════════════════════════════════════════════════════════════════════════╝
"#
        );
    }

    fn show_quantum_help() {
        println!(
            r#"
╔══════════════════════════════════════════════════════════════════════════════╗
║                          QUANTUM COMPUTING                                   ║
╠══════════════════════════════════════════════════════════════════════════════╣
║                                                                              ║
║  COMMANDS                                                                    ║
║    housaky quantum status            Show quantum backend status            ║
║    housaky quantum bench             Run benchmarks                          ║
║    housaky quantum solve <problem>   Solve optimization problem            ║
║                                                                              ║
║  BACKENDS                                                                    ║
║    • simulator     - Local simulator (free, default)                        ║
║    • braket       - Amazon Braket (requires AWS config)                     ║
║                                                                              ║
║  USE CASES                                                                   ║
║    • Goal scheduling optimization                                              ║
║    • Memory graph optimization                                                ║
║    • Reasoning search (Grover's algorithm)                                   ║
║    • Hybrid classical-quantum solving                                       ║
║                                                                              ║
╚══════════════════════════════════════════════════════════════════════════════╝
"#
        );
    }

    fn show_all_help() {
        Self::show_tips();
        println!();
        Self::show_commands();
    }
}

pub fn run_tui_command(
    name: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    _temperature: f64,
    config: Config,
) -> anyhow::Result<()> {
    let tui_name = name.unwrap_or_else(|| "chat".to_string());

    match tui_name.to_lowercase().as_str() {
        "chat" | "c" => crate::tui::run_chat_tui(config, provider, model, None),
        "skills" | "skill" | "s" => {
            let repo_root =
                std::env::current_dir().unwrap_or_else(|_| config.workspace_dir.clone());
            crate::tui::run_skills_market_tui(config, repo_root)
        }
        "keys" | "key" | "k" => {
            println!("Keys are managed via CLI: housaky keys <command>");
            println!("Run 'housaky help keys' for more info.");
            Ok(())
        }
        "config" | "cfg" => {
            println!("Config editor: housaky config edit");
            Ok(())
        }
        "doctor" | "diag" | "d" => {
            crate::cli::handle_doctor(&config, Some(crate::commands::DoctorCommands::Run))
        }
        "agi" | "a" => crate::tui::run_agi_tui(config, provider, model, None),
        "live" | "thoughts" | "t" => crate::tui::live::run_live_agi_tui(config, provider, model),
        "commands" | "cmd" | "palette" => {
            println!("Command palette: Ctrl+K in any TUI");
            Ok(())
        }
        "help" | "h" => {
            HelpSystem::show_help(None);
            Ok(())
        }
        _ => {
            println!("Unknown TUI: {}", tui_name);
            println!("Available TUIs: chat, skills, keys, config, doctor, agi, live, commands");
            Ok(())
        }
    }
}
