import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { createStore } from 'vuex'
import VueRouter from 'vue-router'
import vuetify from './plugins/vuetify'
import App from './App.vue'
import router from './router'
import store from './store'
import './styles/psychedelic.scss'
import 'animate.css'

const app = createApp(App)

// Use Pinia for state management
app.use(createPinia())

// Use Vuex for complex state
app.use(store)

// Use Vue Router
app.use(router)

// Use Vuetify
app.use(vuetify)

app.mount('#app')
