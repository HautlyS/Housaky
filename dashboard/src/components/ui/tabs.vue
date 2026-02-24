<script setup lang="ts">
import { provide, ref } from 'vue'

const activeTab = ref('overview')

provide('activeTab', activeTab)

defineProps<{
  defaultValue?: string
}>()
</script>

<template>
  <div class="w-full">
    <slot />
  </div>
</template>
