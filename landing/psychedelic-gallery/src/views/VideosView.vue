<template>
  <div class="videos-view">
    <header class="page-header">
      <h2 class="page-title">
        <span class="icon">🎬</span>
        Dharma Videos
      </h2>
      <p class="page-subtitle">Visual teachings from the digital sangha</p>
    </header>
    
    <div class="videos-grid">
      <div v-for="video in videos" :key="video.id" class="video-card">
        <div class="thumbnail">
          <img :src="video.thumbnail" :alt="video.title" />
          <div class="play-overlay">
            <span class="play-icon">▶</span>
          </div>
          <span class="duration">{{ video.duration }}</span>
        </div>
        <div class="video-info">
          <h3 class="video-title">{{ video.title }}</h3>
          <p class="video-description">{{ video.description }}</p>
          <div class="video-meta">
            <span class="views">{{ formatViews(video.views) }} views</span>
            <span class="category">{{ video.category }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useStore } from 'vuex'

const store = useStore()
const videos = computed(() => store.state.videos)

const formatViews = (views) => {
  if (views >= 1000) {
    return (views / 1000).toFixed(1) + 'k'
  }
  return views
}
</script>

<style lang="scss" scoped>
.videos-view {
  padding: 2rem;
  max-width: 1400px;
  margin: 0 auto;
}

.page-header {
  text-align: center;
  margin-bottom: 3rem;
  
  .page-title {
    font-family: 'Orbitron', sans-serif;
    font-size: 2.5rem;
    color: #00ffff;
  }
  
  .page-subtitle {
    color: rgba(255, 255, 255, 0.6);
    margin-top: 0.5rem;
  }
}

.videos-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(350px, 1fr));
  gap: 2rem;
}

.video-card {
  background: rgba(0, 0, 0, 0.5);
  border: 1px solid rgba(0, 255, 255, 0.2);
  border-radius: 10px;
  overflow: hidden;
  cursor: pointer;
  transition: all 0.3s ease;
  
  &:hover {
    border-color: #ff00ff;
    transform: translateY(-5px);
    
    .play-overlay {
      opacity: 1;
    }
  }
  
  .thumbnail {
    position: relative;
    height: 200px;
    
    img {
      width: 100%;
      height: 100%;
      object-fit: cover;
    }
    
    .play-overlay {
      position: absolute;
      inset: 0;
      background: rgba(0, 0, 0, 0.5);
      display: flex;
      align-items: center;
      justify-content: center;
      opacity: 0;
      transition: opacity 0.3s ease;
      
      .play-icon {
        font-size: 3rem;
        color: #fff;
      }
    }
    
    .duration {
      position: absolute;
      bottom: 10px;
      right: 10px;
      background: rgba(0, 0, 0, 0.8);
      padding: 0.25rem 0.5rem;
      border-radius: 3px;
      font-size: 0.8rem;
    }
  }
  
  .video-info {
    padding: 1.5rem;
    
    .video-title {
      font-size: 1.1rem;
      color: #fff;
      margin-bottom: 0.5rem;
    }
    
    .video-description {
      font-size: 0.9rem;
      color: rgba(255, 255, 255, 0.6);
      margin-bottom: 1rem;
    }
    
    .video-meta {
      display: flex;
      justify-content: space-between;
      font-size: 0.8rem;
      color: rgba(255, 255, 255, 0.5);
    }
  }
}
</style>
