import { createRouter, createWebHistory } from 'vue-router'

const routes = [
  {
    path: '/',
    name: 'Home',
    component: () => import('@/views/HomeView.vue')
  },
  {
    path: '/articles',
    name: 'Articles',
    component: () => import('@/views/ArticlesView.vue')
  },
  {
    path: '/articles/:id',
    name: 'ArticleDetail',
    component: () => import('@/views/ArticleDetailView.vue')
  },
  {
    path: '/videos',
    name: 'Videos',
    component: () => import('@/views/VideosView.vue')
  },
  {
    path: '/teachings',
    name: 'Teachings',
    component: () => import('@/views/TeachingsView.vue')
  },
  {
    path: '/meditation',
    name: 'Meditation',
    component: () => import('@/views/MeditationView.vue')
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes,
  scrollBehavior(to, from, savedPosition) {
    if (savedPosition) {
      return savedPosition
    } else {
      return { top: 0, behavior: 'smooth' }
    }
  }
})

export default router
