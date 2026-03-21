//! Terminal markdown renderer with ANSI colors and styling.
//! Converts markdown to beautifully formatted terminal output.

use std::io::Write;

/// ANSI color codes
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const ITALIC: &str = "\x1b[3m";
    pub const UNDERLINE: &str = "\x1b[4m";

    pub const BLACK: &str = "\x1b[30m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";

    pub const BRIGHT_BLACK: &str = "\x1b[90m";
    pub const BRIGHT_RED: &str = "\x1b[91m";
    pub const BRIGHT_GREEN: &str = "\x1b[92m";
    pub const BRIGHT_YELLOW: &str = "\x1b[93m";
    pub const BRIGHT_BLUE: &str = "\x1b[94m";
    pub const BRIGHT_MAGENTA: &str = "\x1b[95m";
    pub const BRIGHT_CYAN: &str = "\x1b[96m";
    pub const BRIGHT_WHITE: &str = "\x1b[97m";

    pub const BG_BLACK: &str = "\x1b[40m";
    pub const BG_RED: &str = "\x1b[41m";
    pub const BG_GREEN: &str = "\x1b[42m";
    pub const BG_YELLOW: &str = "\x1b[43m";
    pub const BG_BLUE: &str = "\x1b[44m";
    pub const BG_MAGENTA: &str = "\x1b[45m";
    pub const BG_CYAN: &str = "\x1b[46m";
    pub const BG_WHITE: &str = "\x1b[47m";
}

use colors::*;

/// Styled output configuration
pub struct StyledOutput {
    pub user_color: &'static str,
    pub agent_color: &'static str,
    pub system_color: &'static str,
    pub error_color: &'static str,
    pub code_bg: &'static str,
    pub link_color: &'static str,
    pub quote_color: &'static str,
    pub header_color: &'static str,
}

impl Default for StyledOutput {
    fn default() -> Self {
        Self {
            user_color: BRIGHT_CYAN,
            agent_color: BRIGHT_GREEN,
            system_color: BRIGHT_YELLOW,
            error_color: BRIGHT_RED,
            code_bg: BG_BLACK,
            link_color: BRIGHT_BLUE,
            quote_color: BRIGHT_BLACK,
            header_color: BRIGHT_MAGENTA,
        }
    }
}

impl StyledOutput {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Render markdown text to styled terminal output
pub fn render_markdown(text: &str) -> String {
    render_markdown_with_config(text, &StyledOutput::default())
}

/// Render markdown with custom color configuration
pub fn render_markdown_with_config(text: &str, config: &StyledOutput) -> String {
    let mut result = String::new();
    let mut in_code_block = false;
    let mut in_inline_code = false;
    let mut code_lang = String::new();
    let mut chars = text.chars().peekable();
    let mut line_start = true;

    while let Some(ch) = chars.next() {
        // Handle code blocks
        if ch == '`' {
            let count = count_backticks(&mut chars);

            if count >= 2 {
                // Code block start/end
                if in_code_block {
                    result.push_str(RESET);
                    result.push_str("\n");
                    in_code_block = false;
                } else {
                    // Extract language if present
                    code_lang.clear();
                    while let Some(&next) = chars.peek() {
                        if next == '\n' || next == ' ' {
                            break;
                        }
                        code_lang.push(chars.next().unwrap());
                    }

                    result.push_str("\n");
                    result.push_str(DIM);
                    result.push_str("┌─");
                    if !code_lang.is_empty() {
                        result.push_str(" ");
                        result.push_str(&code_lang);
                    }
                    result.push_str(" ─");
                    result.push_str(RESET);
                    result.push_str("\n");
                    result.push_str(DIM);
                    result.push_str("│ ");
                    result.push_str(RESET);
                    in_code_block = true;
                }
                continue;
            } else if count == 1 {
                // Inline code
                if in_inline_code {
                    result.push_str(RESET);
                    in_inline_code = false;
                } else {
                    result.push_str(config.code_bg);
                    result.push_str(YELLOW);
                    in_inline_code = true;
                }
                continue;
            }
        }

        // Handle newlines in code blocks
        if in_code_block && ch == '\n' {
            result.push_str(RESET);
            result.push_str("\n");
            result.push_str(DIM);
            result.push_str("│ ");
            result.push_str(RESET);
            continue;
        }

        // Handle line start markers
        if line_start {
            // Headers
            if ch == '#' {
                let mut level = 1;
                while let Some(&'#') = chars.peek() {
                    chars.next();
                    level += 1;
                }
                // Skip space after #
                if let Some(&' ') = chars.peek() {
                    chars.next();
                }

                let prefix = match level {
                    1 => "█ ",
                    2 => "▓ ",
                    3 => "▒ ",
                    4 => "░ ",
                    _ => "• ",
                };

                result.push_str("\n");
                result.push_str(BOLD);
                result.push_str(config.header_color);
                result.push_str(prefix);
                continue;
            }

            // Quote
            if ch == '>' {
                if let Some(&' ') = chars.peek() {
                    chars.next();
                }
                result.push_str(DIM);
                result.push_str("│ ");
                result.push_str(config.quote_color);
                result.push_str(ITALIC);
                continue;
            }

            // List items
            if ch == '-' || ch == '*' || ch == '+' {
                if let Some(&' ') = chars.peek() {
                    chars.next();
                    result.push_str(DIM);
                    result.push_str("• ");
                    result.push_str(RESET);
                    continue;
                }
            }

            // Numbered list
            if ch.is_ascii_digit() {
                let mut num = String::new();
                num.push(ch);
                while let Some(&d) = chars.peek() {
                    if d.is_ascii_digit() {
                        num.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if let Some(&'.') = chars.peek() {
                    chars.next();
                    if let Some(&' ') = chars.peek() {
                        chars.next();
                    }
                    result.push_str(DIM);
                    result.push_str(&num);
                    result.push_str(". ");
                    result.push_str(RESET);
                    continue;
                }
                result.push_str(&num);
                continue;
            }

            // Horizontal rule
            if ch == '-' || ch == '*' || ch == '_' {
                let mut hr_count = 1;
                let hr_char = ch;
                while let Some(&c) = chars.peek() {
                    if c == hr_char {
                        hr_count += 1;
                        chars.next();
                    } else if c == ' ' {
                        chars.next();
                    } else {
                        break;
                    }
                }
                if hr_count >= 3 {
                    result.push_str(DIM);
                    result.push_str("────────────────────────────────────────");
                    result.push_str(RESET);
                    result.push('\n');
                    continue;
                }
                result.push(ch);
                continue;
            }
        }

        // Bold **text**
        if ch == '*' {
            if let Some(&'*') = chars.peek() {
                chars.next();
                if let Some(&next) = chars.peek() {
                    if next != '*' && next != ' ' {
                        result.push_str(BOLD);
                        continue;
                    }
                }
                result.push_str(RESET);
                continue;
            }
        }

        // Italic *text* or _text_
        if ch == '*' || ch == '_' {
            if let Some(&next) = chars.peek() {
                if next != '*' && next != ' ' && next != '\n' {
                    result.push_str(ITALIC);
                    continue;
                }
            }
        }

        // Links [text](url)
        if ch == '[' {
            let mut link_text = String::new();
            let mut found_url = false;
            let mut url = String::new();

            // Collect link text
            while let Some(&next) = chars.peek() {
                if next == ']' {
                    chars.next();
                    break;
                }
                link_text.push(chars.next().unwrap());
            }

            // Check for URL
            if let Some(&'(') = chars.peek() {
                chars.next();
                while let Some(&next) = chars.peek() {
                    if next == ')' {
                        chars.next();
                        found_url = true;
                        break;
                    }
                    url.push(chars.next().unwrap());
                }
            }

            if found_url {
                result.push_str(config.link_color);
                result.push_str(UNDERLINE);
                result.push_str(&link_text);
                result.push_str(RESET);
                result.push_str(DIM);
                result.push_str(" (");
                result.push_str(&url);
                result.push_str(")");
                result.push_str(RESET);
                continue;
            }
            result.push('[');
            result.push_str(&link_text);
            continue;
        }

        // Strikethrough ~~text~~
        if ch == '~' {
            if let Some(&'~') = chars.peek() {
                chars.next();
                result.push_str(DIM);
                result.push_str("\u{0336}"); // Combining strikethrough
                continue;
            }
        }

        // Track line start
        if ch == '\n' {
            line_start = true;
            result.push_str(RESET);
            result.push('\n');
            continue;
        }

        line_start = false;
        result.push(ch);
    }

    // Close any open formatting
    result.push_str(RESET);

    result
}

/// Count consecutive backticks from iterator
fn count_backticks(chars: &mut std::iter::Peekable<std::str::Chars>) -> usize {
    let mut count = 0;
    while let Some(&'`') = chars.peek() {
        chars.next();
        count += 1;
    }
    count
}

/// Print a styled user message
pub fn print_user_message(message: &str, config: &StyledOutput) {
    let timestamp = chrono::Local::now().format("%H:%M:%S");
    print!(
        "{}[{}] {}{}{}You:{} ",
        DIM, timestamp, RESET, config.user_color, BOLD, RESET
    );
    println!("{}", config.user_color);
    println!("  {}", render_markdown_with_config(message, config));
    println!("{}", RESET);
}

/// Print a styled agent message
pub fn print_agent_message(message: &str, config: &StyledOutput) {
    let timestamp = chrono::Local::now().format("%H:%M:%S");
    print!(
        "{}[{}] {}{}{}Housaky:{} ",
        DIM, timestamp, RESET, config.agent_color, BOLD, RESET
    );
    println!("{}", config.agent_color);
    let rendered = render_markdown_with_config(message, config);
    for line in rendered.lines() {
        println!("  {}", line);
    }
    println!("{}", RESET);
}

/// Print a styled system message
pub fn print_system_message(message: &str, config: &StyledOutput) {
    let timestamp = chrono::Local::now().format("%H:%M:%S");
    println!(
        "{}[{}] {}{}{}",
        DIM, timestamp, RESET, config.system_color, message
    );
}

/// Print an error message
pub fn print_error(message: &str, config: &StyledOutput) {
    eprintln!("{}Error: {}", config.error_color, message);
}

/// Print a styled banner
pub fn print_banner() {
    println!();
    println!(
        "{}╭──────────────────────────────────────────────────────────╮",
        CYAN
    );
    println!(
        "│  {}Housaky{} {}AI Assistant{}  │",
        BRIGHT_CYAN, "", BRIGHT_GREEN, ""
    );
    println!("│  {}Type /help for commands, /quit to exit{}  │", DIM, "");
    println!(
        "{}╰──────────────────────────────────────────────────────────╯",
        CYAN
    );
    println!();
}

/// Print the prompt
pub fn print_prompt(config: &StyledOutput) {
    print!(
        "{}{}>{}{} ",
        BOLD, config.user_color, RESET, config.user_color
    );
    let _ = std::io::stdout().flush();
}

/// Print help message
pub fn print_help() {
    println!();
    println!("╭─ Commands ──────────────────────────────────────────────╮");
    println!("│  /help, /h, /?     Show this help              │");
    println!("│  /quit, /q, /exit Exit the chat               │");
    println!("│  /clear, /cl       Clear screen                │");
    println!("│  /provider <name>  Switch LLM provider         │");
    println!("│  /model <name>     Switch model                │");
    println!("│  /multi <text>     Multi-line input mode       │");
    println!("╰────────────────────────────────────────────────────────╯");
    println!();
}

/// Print status info
pub fn print_status(provider: &str, model: &str) {
    println!("Provider: {}  Model: {}", provider, model);
    println!();
}

/// Print a code block with syntax highlighting hint
pub fn print_code_block(code: &str, lang: Option<&str>, config: &StyledOutput) {
    println!();
    println!(
        "{}{}┌─{} {}{}{}{}{}─{}┐",
        DIM,
        config.code_bg,
        RESET,
        BRIGHT_YELLOW,
        lang.unwrap_or("code"),
        RESET,
        DIM,
        "─".repeat(50 - lang.unwrap_or("code").len().min(50)),
        config.code_bg
    );
    println!("{}│{}{} {}", DIM, config.code_bg, RESET, YELLOW);
    for line in code.lines() {
        println!("{}│{}{} {}{}", DIM, config.code_bg, RESET, YELLOW, line);
    }
    println!(
        "{}{}└{}{}{}",
        DIM,
        config.code_bg,
        "─".repeat(60),
        RESET,
        RESET
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_bold() {
        let input = "This is **bold** text";
        let output = render_markdown(input);
        assert!(output.contains("\x1b[1m"));
        assert!(output.contains("\x1b[0m"));
    }

    #[test]
    fn test_render_italic() {
        let input = "This is *italic* text";
        let output = render_markdown(input);
        assert!(output.contains("\x1b[3m"));
    }

    #[test]
    fn test_render_code() {
        let input = "Use `code` here";
        let output = render_markdown(input);
        assert!(output.contains("\x1b[33m")); // Yellow
    }

    #[test]
    fn test_render_header() {
        let input = "# Header\n## Subheader";
        let output = render_markdown(input);
        assert!(output.contains("█"));
        assert!(output.contains("▓"));
    }

    #[test]
    fn test_render_list() {
        let input = "- Item 1\n- Item 2\n1. Numbered";
        let output = render_markdown(input);
        assert!(output.contains("•"));
    }

    #[test]
    fn test_render_link() {
        let input = "Check [link](https://example.com) here";
        let output = render_markdown(input);
        assert!(output.contains("link"));
        assert!(output.contains("https://example.com"));
    }

    #[test]
    fn test_render_quote() {
        let input = "> This is a quote";
        let output = render_markdown(input);
        assert!(output.contains("│"));
    }
}
