import { createRouter, createWebHistory } from 'vue-router'
import Landing from '../views/Landing.vue'
import Home from '../a2a/views/Home.vue'
import Instances from '../a2a/views/Instances.vue'
import Memory from '../a2a/views/Memory.vue'
import Research from '../a2a/views/Research.vue'
import A2A from '../a2a/views/A2A.vue'
import Nodes from '../a2a/views/Nodes.vue'
import Terminal from '../a2a/views/Terminal.vue'
import Security from '../a2a/views/Security.vue'

const base = import.meta.env.BASE_URL.replace(/\/$/, '')
const a2a = base + '/A2A'

const routes = [
  {
    path: base + '/',
    name: 'Landing',
    component: Landing
  },
  {
    path: a2a + '/',
    name: 'A2AHome',
    component: Home,
    meta: { a2a: true }
  },
  {
    path: a2a + '/instances',
    name: 'A2AInstances',
    component: Instances,
    meta: { a2a: true }
  },
  {
    path: a2a + '/memory',
    name: 'A2AMemory',
    component: Memory,
    meta: { a2a: true }
  },
  {
    path: a2a + '/research',
    name: 'A2AResearch',
    component: Research,
    meta: { a2a: true }
  },
  {
    path: a2a + '/a2a',
    name: 'A2A',
    component: A2A,
    meta: { a2a: true }
  },
  {
    path: a2a + '/nodes',
    name: 'A2ANodes',
    component: Nodes,
    meta: { a2a: true }
  },
  {
    path: a2a + '/terminal',
    name: 'A2ATerminal',
    component: Terminal,
    meta: { a2a: true }
  },
  {
    path: a2a + '/security',
    name: 'A2ASecurity',
    component: Security,
    meta: { a2a: true }
  }
]

const router = createRouter({
  history: createWebHistory(base),
  routes
})

export default router
