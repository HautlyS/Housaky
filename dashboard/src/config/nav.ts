import { type Component } from 'vue'
import {
  LayoutDashboard,
  MessageSquare,
  Settings,
  Wrench,
  Network,
  Package,
  Cpu,
  Terminal,
  Brain,
  Shield,
  Wifi,
  Lock,
  Key,
  Bot,
  Server,
} from 'lucide-vue-next'

export interface NavItem {
  title: string
  icon: Component
  path: string
  badge?: string
  category?: string
}

export const navItems: NavItem[] = [
  { title: 'Dashboard', icon: LayoutDashboard, path: '/' },
  { title: 'Chat', icon: MessageSquare, path: '/chat' },
  { title: 'AGI', icon: Brain, path: '/agi', badge: 'live' },
  { title: 'Skills', icon: Wrench, path: '/skills' },
  { title: 'MCP Servers', icon: Server, path: '/mcp' },
  { title: 'Channels', icon: Network, path: '/channels' },
  { title: 'Integrations', icon: Package, path: '/integrations' },
  { title: 'Hardware', icon: Cpu, path: '/hardware' },
  { title: 'A2A Network', icon: Wifi, path: '/a2a-instances', category: 'A2A' },
  { title: 'A2A Messages', icon: MessageSquare, path: '/a2a-messages', category: 'A2A' },
  { title: 'Security', icon: Shield, path: '/security', category: 'Security' },
  { title: 'Keys & Subagents', icon: Key, path: '/keys', category: 'Config' },
  { title: 'Kowalski', icon: Bot, path: '/kowalski', category: 'Agents' },
  { title: 'Config', icon: Settings, path: '/config' },
  { title: 'Terminal', icon: Terminal, path: '/terminal' },
]
