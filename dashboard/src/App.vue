<script setup lang="ts">
import { RouterView, useRoute, useRouter } from 'vue-router'
import { navItems } from '@/config/nav'
import { cn } from '@/lib/utils'
import Button from '@/components/ui/button.vue'
import ModeToggle from '@/components/ui/theme-toggle.vue'

const route = useRoute()
const router = useRouter()
</script>

<template>
  <div class="min-h-screen bg-background">
    <div class="flex h-screen overflow-hidden">
      <!-- Sidebar -->
      <aside class="w-64 border-r bg-card flex flex-col">
        <div class="p-4 border-b">
          <div class="flex items-center gap-2">
            <div class="w-8 h-8 rounded-lg bg-primary flex items-center justify-center">
              <span class="text-primary-foreground font-bold">H</span>
            </div>
            <div>
              <h1 class="font-bold text-lg">Housaky</h1>
              <p class="text-xs text-muted-foreground">AI Assistant</p>
            </div>
          </div>
        </div>
        
        <nav class="flex-1 p-2 space-y-1 overflow-y-auto">
          <RouterLink
            v-for="item in navItems"
            :key="item.path"
            :to="item.path"
            :class="cn(
              'flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors',
              route.path === item.path
                ? 'bg-primary text-primary-foreground'
                : 'text-muted-foreground hover:bg-muted hover:text-foreground'
            )"
          >
            <component :is="item.icon" class="w-5 h-5" />
            {{ item.title }}
          </RouterLink>
        </nav>
        
        <div class="p-4 border-t">
          <ModeToggle />
        </div>
      </aside>
      
      <!-- Main content -->
      <main class="flex-1 overflow-y-auto">
        <RouterView />
      </main>
    </div>
  </div>
</template>
