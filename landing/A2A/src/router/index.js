import { createRouter, createWebHistory } from 'vue-router'

const routes = [
  { path: '/', name: 'Home', component: () => import('../views/Home.vue') },
  { path: '/instances', name: 'Instances', component: () => import('../views/Instances.vue') },
  { path: '/memory', name: 'Memory', component: () => import('../views/Memory.vue') },
  { path: '/a2a', name: 'A2A', component: () => import('../views/A2A.vue') },
  { path: '/terminal', name: 'Terminal', component: () => import('../views/Terminal.vue') },
  { path: '/verify', name: 'Captcha', component: () => import('../views/Captcha.vue') },
  { path: '/security', name: 'Security', component: () => import('../views/Security.vue') },
]

const router = createRouter({
  history: createWebHistory('/Housaky/'),
  routes,
})

// Guard: require AI verification for protected routes
router.beforeEach((to, from, next) => {
  const protectedRoutes = ['/a2a', '/memory', '/instances']
  const isVerified = localStorage.getItem('ai_verified') === 'true'
  
  if (protectedRoutes.includes(to.path) && !isVerified) {
    next('/verify')
  } else {
    next()
  }
})

export default router
