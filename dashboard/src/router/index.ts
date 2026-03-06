import { createRouter, createWebHistory } from 'vue-router'
import DashboardView from '@/views/DashboardView.vue'
import ChatView from '@/views/ChatView.vue'
import AGIView from '@/views/AGIView.vue'
import SkillsView from '@/views/SkillsView.vue'
import ChannelsView from '@/views/ChannelsView.vue'
import IntegrationsView from '@/views/IntegrationsView.vue'
import HardwareView from '@/views/HardwareView.vue'
import ConfigView from '@/views/ConfigView.vue'
import TerminalView from '@/views/TerminalView.vue'
import A2AInstancesView from '@/views/A2AInstancesView.vue'
import A2AMessagesView from '@/views/A2AMessagesView.vue'
import SecurityView from '@/views/SecurityView.vue'
import AIProveView from '@/views/AIProveView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'dashboard',
      component: DashboardView,
    },
    {
      path: '/chat',
      name: 'chat',
      component: ChatView,
    },
    {
      path: '/agi',
      name: 'agi',
      component: AGIView,
    },
    {
      path: '/skills',
      name: 'skills',
      component: SkillsView,
    },
    {
      path: '/channels',
      name: 'channels',
      component: ChannelsView,
    },
    {
      path: '/integrations',
      name: 'integrations',
      component: IntegrationsView,
    },
    {
      path: '/hardware',
      name: 'hardware',
      component: HardwareView,
    },
    {
      path: '/config',
      name: 'config',
      component: ConfigView,
    },
    {
      path: '/terminal',
      name: 'terminal',
      component: TerminalView,
    },
    {
      path: '/a2a-instances',
      name: 'a2a-instances',
      component: A2AInstancesView,
    },
    {
      path: '/a2a-messages',
      name: 'a2a-messages',
      component: A2AMessagesView,
    },
    {
      path: '/security',
      name: 'security',
      component: SecurityView,
    },
    {
      path: '/ai-prove',
      name: 'ai-prove',
      component: AIProveView,
    },
  ],
})

export default router
