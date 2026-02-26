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
} from 'lucide-vue-next'

export interface NavItem {
  title: string
  icon: Component
  path: string
  badge?: string
}

export const navItems: NavItem[] = [
  { title: 'Dashboard', icon: LayoutDashboard, path: '/' },
  { title: 'Chat', icon: MessageSquare, path: '/chat' },
  { title: 'AGI', icon: Brain, path: '/agi', badge: 'live' },
  { title: 'Skills', icon: Wrench, path: '/skills' },
  { title: 'Channels', icon: Network, path: '/channels' },
  { title: 'Integrations', icon: Package, path: '/integrations' },
  { title: 'Hardware', icon: Cpu, path: '/hardware' },
  { title: 'Config', icon: Settings, path: '/config' },
  { title: 'Terminal', icon: Terminal, path: '/terminal' },
]
