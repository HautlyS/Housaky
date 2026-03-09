import { createRouter, createWebHistory } from 'vue-router'

const routes = [
  {
    path: '/',
    name: 'Home',
    component: () => import('../views/Home.vue')
  },
  {
    path: '/instances',
    name: 'Instances',
    component: () => import('../views/Instances.vue')
  },
  {
    path: '/memory',
    name: 'Memory',
    component: () => import('../views/Memory.vue')
  },
  {
    path: '/a2a',
    name: 'A2A',
    component: () => import('../views/A2A.vue')
  },
  {
    path: '/nodes',
    name: 'Nodes',
    component: () => import('../views/Nodes.vue')
  },
  {
    path: '/security',
    name: 'Security',
    component: () => import('../views/Security.vue')
  },
  {
    path: '/terminal',
    name: 'Terminal',
    component: () => import('../views/Terminal.vue')
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

export default router
