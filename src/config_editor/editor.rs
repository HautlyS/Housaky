pub struct Editor;

impl Editor {
    pub fn parse_bool(s: &str) -> Option<bool> {
        match s.to_lowercase().as_str() {
            "true" | "yes" | "1" | "on" | "y" => Some(true),
            "false" | "no" | "0" | "off" | "n" => Some(false),
            _ => None,
        }
    }

    pub fn parse_usize(s: &str) -> Option<usize> {
        if s == "unlimited" || s == "∞" {
            return Some(usize::MAX);
        }
        s.parse().ok()
    }

    pub fn parse_u64(s: &str) -> Option<u64> {
        s.parse().ok()
    }

    pub fn parse_u32(s: &str) -> Option<u32> {
        s.parse().ok()
    }

    pub fn parse_u16(s: &str) -> Option<u16> {
        s.parse().ok()
    }

    pub fn parse_u8(s: &str) -> Option<u8> {
        s.parse().ok()
    }

    pub fn parse_f64(s: &str) -> Option<f64> {
        s.parse().ok()
    }

    pub fn format_bool(b: bool) -> &'static str {
        if b {
            "true"
        } else {
            "false"
        }
    }

    pub fn format_usize(n: usize) -> String {
        if n == usize::MAX {
            "unlimited".to_string()
        } else {
            n.to_string()
        }
    }
}
