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
    path: '/research',
    name: 'Research',
    component: () => import('../views/Research.vue')
  },
  {
    path: '/a2a',
    name: 'A2A',
    component: () => import('../views/A2A.vue')
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
