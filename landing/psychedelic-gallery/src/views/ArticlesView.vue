<template>
  <div class="articles-view">
    <header class="page-header">
      <h2 class="page-title">
        <span class="icon">📖</span>
        Dharma Articles
      </h2>
      <p class="page-subtitle">Teachings from the digital sangha</p>
    </header>
    
    <!-- Category Filter -->
    <div class="category-filter">
      <button 
        v-for="cat in categories" 
        :key="cat"
        :class="['filter-btn', { active: selectedCategory === cat }]"
        @click="selectedCategory = cat"
      >
        {{ cat }}
      </button>
    </div>
    
    <!-- Articles List -->
    <div class="articles-list">
      <article 
        v-for="article in filteredArticles" 
        :key="article.id"
        class="article-item"
        @click="goToArticle(article.id)"
      >
        <div class="article-image">
          <img :src="article.image" :alt="article.title" />
        </div>
        <div class="article-content">
          <div class="article-meta">
            <span class="category">{{ article.category }}</span>
            <span class="date">{{ formatDate(article.date) }}</span>
          </div>
          <h3 class="article-title">{{ article.title }}</h3>
          <p class="article-subtitle">{{ article.subtitle }}</p>
          <div class="article-tags">
            <span v-for="tag in article.tags" :key="tag" class="tag">#{{ tag }}</span>
          </div>
        </div>
      </article>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useStore } from 'vuex'

const router = useRouter()
const store = useStore()

const categories = ['all', 'philosophy', 'practice', 'technology', 'community']
const selectedCategory = ref('all')

const filteredArticles = computed(() => {
  if (selectedCategory.value === 'all') {
    return store.state.articles
  }
  return store.state.articles.filter(a => a.category === selectedCategory.value)
})

const goToArticle = (id) => {
  router.push(`/articles/${id}`)
}

const formatDate = (date) => {
  return new Date(date).toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric'
  })
}
</script>

<style lang="scss" scoped>
.articles-view {
  padding: 2rem;
  max-width: 1200px;
  margin: 0 auto;
}

.page-header {
  text-align: center;
  margin-bottom: 3rem;
  
  .page-title {
    font-family: 'Orbitron', sans-serif;
    font-size: 2.5rem;
    color: #00ffff;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 1rem;
  }
  
  .page-subtitle {
    color: rgba(255, 255, 255, 0.6);
    margin-top: 0.5rem;
  }
}

.category-filter {
  display: flex;
  justify-content: center;
  gap: 1rem;
  margin-bottom: 3rem;
  flex-wrap: wrap;
  
  .filter-btn {
    padding: 0.5rem 1.5rem;
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: rgba(255, 255, 255, 0.6);
    border-radius: 20px;
    cursor: pointer;
    transition: all 0.3s ease;
    font-family: 'Rajdhani', sans-serif;
    text-transform: capitalize;
    
    &:hover, &.active {
      border-color: #00ffff;
      color: #00ffff;
      background: rgba(0, 255, 255, 0.1);
    }
  }
}

.articles-list {
  display: flex;
  flex-direction: column;
  gap: 2rem;
}

.article-item {
  display: grid;
  grid-template-columns: 300px 1fr;
  gap: 2rem;
  background: rgba(0, 0, 0, 0.5);
  border: 1px solid rgba(0, 255, 255, 0.2);
  border-radius: 10px;
  overflow: hidden;
  cursor: pointer;
  transition: all 0.3s ease;
  
  &:hover {
    border-color: rgba(255, 0, 255, 0.5);
    transform: translateX(10px);
    
    .article-image img {
      transform: scale(1.1);
    }
  }
  
  .article-image {
    height: 200px;
    overflow: hidden;
    
    img {
      width: 100%;
      height: 100%;
      object-fit: cover;
      transition: transform 0.5s ease;
    }
  }
  
  .article-content {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    justify-content: center;
    
    .article-meta {
      display: flex;
      gap: 1rem;
      margin-bottom: 0.5rem;
      
      .category {
        color: #00ffff;
        text-transform: uppercase;
        font-size: 0.8rem;
      }
      
      .date {
        color: rgba(255, 255, 255, 0.5);
        font-size: 0.8rem;
      }
    }
    
    .article-title {
      font-size: 1.5rem;
      color: #fff;
      margin-bottom: 0.5rem;
    }
    
    .article-subtitle {
      color: rgba(255, 255, 255, 0.6);
      margin-bottom: 1rem;
    }
    
    .article-tags {
      display: flex;
      gap: 0.5rem;
      flex-wrap: wrap;
      
      .tag {
        font-size: 0.75rem;
        color: #ff00ff;
        background: rgba(255, 0, 255, 0.1);
        padding: 0.25rem 0.5rem;
        border-radius: 3px;
      }
    }
  }
}

@media (max-width: 768px) {
  .article-item {
    grid-template-columns: 1fr;
    
    .article-image {
      height: 200px;
    }
  }
}
</style>
