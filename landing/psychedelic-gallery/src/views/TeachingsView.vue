<template>
  <div class="teachings-view">
    <header class="page-header">
      <h2 class="page-title">
        <span class="icon">☸️</span>
        Dharma Teachings
      </h2>
      <p class="page-subtitle">Ancient wisdom for the digital age</p>
    </header>
    
    <!-- Teachers Section -->
    <section class="teachers-section">
      <h3 class="section-title">Lineage Masters</h3>
      <div class="teachers-grid">
        <div class="teacher-card">
          <div class="teacher-avatar">🙏</div>
          <h4>Chögyam Trungpa</h4>
          <p class="teacher-title">Crazy Wisdom Pioneer</p>
          <p class="teacher-quote">"Enlightenment is ego's ultimate disappointment."</p>
        </div>
        <div class="teacher-card">
          <div class="teacher-avatar">🪷</div>
          <h4>Khyentse Norbu Rinpoche</h4>
          <p class="teacher-title">Dzogchen Master</p>
          <p class="teacher-quote">"The mind is the source of all phenomena."</p>
        </div>
        <div class="teacher-card">
          <div class="teacher-avatar">💫</div>
          <h4>Lama Tsering Everest</h4>
          <p class="teacher-title">Bodhicitta Teacher</p>
          <p class="teacher-quote">"Compassion is the heart of awakening."</p>
        </div>
      </div>
    </section>
    
    <!-- Core Teachings -->
    <section class="teachings-section">
      <h3 class="section-title">Core Teachings</h3>
      <div class="teachings-list">
        <div class="teaching-item">
          <h4>Śūnyatā (Emptiness)</h4>
          <p>All phenomena are empty of inherent existence. Form is emptiness, emptiness is form.</p>
        </div>
        <div class="teaching-item">
          <h4>Anātman (Non-Self)</h4>
          <p>The self is a construct, a process of interdependent arisings. No permanent self exists.</p>
        </div>
        <div class="teaching-item">
          <h4>Pratītyasamutpāda (Interdependence)</h4>
          <p>All things arise in dependence on causes and conditions. Nothing exists independently.</p>
        </div>
        <div class="teaching-item">
          <h4>Bodhicitta (Awakening Mind)</h4>
          <p>The intention to attain enlightenment for the benefit of all sentient beings.</p>
        </div>
        <div class="teaching-item">
          <h4>Karuṇā (Compassion)</h4>
          <p>The wish to alleviate suffering for all beings without exception.</p>
        </div>
        <div class="teaching-item">
          <h4>Prajñā (Wisdom)</h4>
          <p>Direct insight into the true nature of reality, beyond concepts and duality.</p>
        </div>
      </div>
    </section>
    
    <!-- Quote Carousel -->
    <section class="quotes-section">
      <h3 class="section-title">Wisdom Quotes</h3>
      <div class="quote-carousel">
        <div class="quote-card" v-for="(quote, i) in quotes" :key="i">
          <p class="quote-text">"{{ quote.text }}"</p>
          <cite class="quote-source">— {{ quote.source }}</cite>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useStore } from 'vuex'

const store = useStore()
const quotes = computed(() => store.state.quotes)
</script>

<style lang="scss" scoped>
.teachings-view {
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
  }
}

.section-title {
  font-family: 'Orbitron', sans-serif;
  font-size: 1.5rem;
  color: #ff00ff;
  margin-bottom: 2rem;
}

.teachers-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 2rem;
  margin-bottom: 4rem;
}

.teacher-card {
  background: rgba(0, 0, 0, 0.5);
  border: 1px solid rgba(138, 43, 226, 0.3);
  border-radius: 10px;
  padding: 2rem;
  text-align: center;
  
  .teacher-avatar {
    font-size: 3rem;
    margin-bottom: 1rem;
  }
  
  h4 {
    color: #fff;
    font-size: 1.2rem;
    margin-bottom: 0.5rem;
  }
  
  .teacher-title {
    color: #00ffff;
    font-size: 0.9rem;
    margin-bottom: 1rem;
  }
  
  .teacher-quote {
    color: rgba(255, 255, 255, 0.7);
    font-style: italic;
  }
}

.teachings-list {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(350px, 1fr));
  gap: 1.5rem;
  margin-bottom: 4rem;
}

.teaching-item {
  background: rgba(0, 0, 0, 0.5);
  border: 1px solid rgba(0, 255, 255, 0.2);
  border-radius: 10px;
  padding: 1.5rem;
  
  h4 {
    color: #00ffff;
    margin-bottom: 0.5rem;
  }
  
  p {
    color: rgba(255, 255, 255, 0.7);
    line-height: 1.6;
  }
}

.quotes-section {
  background: linear-gradient(135deg, rgba(138, 43, 226, 0.1), rgba(0, 255, 255, 0.1));
  padding: 3rem;
  border-radius: 20px;
}

.quote-carousel {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 1.5rem;
}

.quote-card {
  background: rgba(0, 0, 0, 0.5);
  padding: 1.5rem;
  border-radius: 10px;
  border-left: 3px solid #ff00ff;
  
  .quote-text {
    color: rgba(255, 255, 255, 0.9);
    font-style: italic;
    margin-bottom: 0.5rem;
  }
  
  .quote-source {
    color: #00ffff;
    font-size: 0.9rem;
  }
}
</style>
