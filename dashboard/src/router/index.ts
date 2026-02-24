import { createRouter, createWebHistory } from 'vue-router'
import DashboardView from '@/views/DashboardView.vue'
import ChatView from '@/views/ChatView.vue'
import SkillsView from '@/views/SkillsView.vue'
import ChannelsView from '@/views/ChannelsView.vue'
import IntegrationsView from '@/views/IntegrationsView.vue'
import HardwareView from '@/views/HardwareView.vue'
import ConfigView from '@/views/ConfigView.vue'
import TerminalView from '@/views/TerminalView.vue'

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
  ],
})

export default router
