import 'vite-plugin-vuetify/styles'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
import '@mdi/font/css/materialdesignicons.css'

export default createVuetify({
  components,
  directives,
  theme: {
    defaultTheme: 'dark',
    themes: {
      dark: {
        colors: {
          primary: '#00ffff',
          secondary: '#ff00ff',
          accent: '#8a2be2',
          background: '#0a0a0a',
          surface: '#1a1a1a',
          error: '#ff0066',
          success: '#00ff66'
        }
      }
    }
  }
})
