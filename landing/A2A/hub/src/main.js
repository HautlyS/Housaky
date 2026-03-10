import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import './styles/main.css'
import './styles/ascii.css'

const app = createApp(App)

app.use(createPinia())
app.use(router)

app.mount('#app')
// Build refresh - 2026-03-10T02:20:11+00:00
