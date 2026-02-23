#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCategory {
    Agent,
    Tools,
    Channels,
    Gateway,
    Memory,
    Providers,
    Fallback,
    Security,
    Cost,
}

impl MenuCategory {
    pub const fn id(&self) -> &'static str {
        match self {
            MenuCategory::Agent => "agent",
            MenuCategory::Tools => "tools",
            MenuCategory::Channels => "channels",
            MenuCategory::Gateway => "gateway",
            MenuCategory::Memory => "memory",
            MenuCategory::Providers => "providers",
            MenuCategory::Fallback => "fallback",
            MenuCategory::Security => "security",
            MenuCategory::Cost => "cost",
        }
    }

    pub const fn label(&self) -> &'static str {
        match self {
            MenuCategory::Agent => "Agent",
            MenuCategory::Tools => "Tools",
            MenuCategory::Channels => "Channels",
            MenuCategory::Gateway => "Gateway",
            MenuCategory::Memory => "Memory",
            MenuCategory::Providers => "Providers",
            MenuCategory::Fallback => "Fallback",
            MenuCategory::Security => "Security",
            MenuCategory::Cost => "Cost",
        }
    }
}

pub const MENU_CATEGORIES: [MenuCategory; 9] = [
    MenuCategory::Agent,
    MenuCategory::Tools,
    MenuCategory::Channels,
    MenuCategory::Gateway,
    MenuCategory::Memory,
    MenuCategory::Providers,
    MenuCategory::Fallback,
    MenuCategory::Security,
    MenuCategory::Cost,
];
