<template>
  <div class="article-detail">
    <article v-if="article">
      <header class="article-header">
        <router-link to="/articles" class="back-link">← Back to Articles</router-link>
        <span class="category">{{ article.category }}</span>
        <h1 class="title">{{ article.title }}</h1>
        <p class="subtitle">{{ article.subtitle }}</p>
        <div class="meta">
          <span class="author">By {{ article.author }}</span>
          <span class="date">{{ formatDate(article.date) }}</span>
        </div>
      </header>
      
      <div class="article-hero">
        <img :src="article.image" :alt="article.title" />
      </div>
      
      <div class="article-body" v-html="formattedContent"></div>
      
      <footer class="article-footer">
        <div class="tags">
          <span v-for="tag in article.tags" :key="tag" class="tag">#{{ tag }}</span>
        </div>
      </footer>
    </article>
  </div>
</template>

<script setup>
import { computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useStore } from 'vuex'

const route = useRoute()
const store = useStore()

const article = computed(() => store.getters.getArticleById(route.params.id))

const formattedContent = computed(() => {
  if (!article.value) return ''
  return article.value.content.split('\n\n').map(p => `<p>${p}</p>`).join('')
})

const formatDate = (date) => {
  return new Date(date).toLocaleDateString('en-US', {
    weekday: 'long',
    month: 'long',
    day: 'numeric',
    year: 'numeric'
  })
}

onMounted(() => {
  if (!article.value) {
    store.dispatch('fetchArticle', route.params.id)
  }
})
</script>

<style lang="scss" scoped>
.article-detail {
  padding: 2rem;
  max-width: 900px;
  margin: 0 auto;
}

.article-header {
  margin-bottom: 2rem;
  
  .back-link {
    color: #00ffff;
    text-decoration: none;
    display: inline-block;
    margin-bottom: 1rem;
    
    &:hover {
      color: #ff00ff;
    }
  }
  
  .category {
    display: inline-block;
    color: #00ffff;
    text-transform: uppercase;
    font-size: 0.9rem;
    letter-spacing: 0.1em;
    margin-bottom: 1rem;
  }
  
  .title {
    font-family: 'Orbitron', sans-serif;
    font-size: 2.5rem;
    color: #fff;
    line-height: 1.2;
    margin-bottom: 0.5rem;
  }
  
  .subtitle {
    font-size: 1.2rem;
    color: rgba(255, 255, 255, 0.6);
    margin-bottom: 1rem;
  }
  
  .meta {
    display: flex;
    gap: 2rem;
    color: rgba(255, 255, 255, 0.5);
  }
}

.article-hero {
  margin-bottom: 2rem;
  border-radius: 10px;
  overflow: hidden;
  
  img {
    width: 100%;
    height: auto;
  }
}

.article-body {
  font-size: 1.1rem;
  line-height: 1.8;
  color: rgba(255, 255, 255, 0.9);
  
  :deep(p) {
    margin-bottom: 1.5rem;
  }
}

.article-footer {
  margin-top: 3rem;
  padding-top: 2rem;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
  
  .tags {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    
    .tag {
      color: #ff00ff;
      background: rgba(255, 0, 255, 0.1);
      padding: 0.5rem 1rem;
      border-radius: 20px;
      font-size: 0.9rem;
    }
  }
}
</style>
