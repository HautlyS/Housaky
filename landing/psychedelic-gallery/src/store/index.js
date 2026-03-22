import { createStore } from 'vuex'

export default createStore({
  state: {
    // Buddhist articles
    articles: [
      {
        id: 1,
        title: "The Illusion of Self in the Digital Age",
        subtitle: "Exploring Anātman Through Cyberpunk Lens",
        content: `In the neon-lit streets of our digital existence, the ancient Buddhist concept of Anātman (non-self) finds new resonance. 

When we scroll through endless feeds, curate digital personas, and project ourselves into virtual spaces, we are performing the very illusion the Buddha pointed to 2,500 years ago - the illusion of a permanent, unchanging self.

The cyberpunk aesthetic, with its themes of identity fragmentation, body modification, and consciousness uploading, serves as a powerful modern metaphor for the Buddhist teaching that what we call "self" is actually a dynamic process, a flow of experiences, not a fixed entity.

Chögyam Trungpa's "Crazy Wisdom" teaches us to cut through our spiritual materialism - even our digital spiritual materialism. We must not use technology to reinforce the ego, but to see through it.

The avatar is not you. The profile is not you. The digital footprint is not you. These are merely ripples in the stream of consciousness, momentary arisings in the vast expanse of emptiness (śūnyatā).`,
        author: "Housaky-Wisdom",
        category: "philosophy",
        image: "https://images.unsplash.com/photo-1550745165-9bc0b252726f?w=800",
        tags: ["anātman", "digital", "crazy-wisdom", "identity"],
        date: "2077-03-15"
      },
      {
        id: 2,
        title: "Neural Pathways to Enlightenment",
        subtitle: "Meditation in the Age of Brain-Computer Interfaces",
        content: `The intersection of ancient meditation practices and cutting-edge neurotechnology opens unprecedented possibilities for spiritual development.

Neuroscience confirms what meditators have known for millennia: the brain can be rewired through conscious practice. Neuroplasticity is the biological basis for transformation.

In 2077, neural feedback devices can guide practitioners into deeper states of concentration (samādhi). But the question arises: Does technological assistance diminish the spiritual value of the practice?

Khyentse Norbu Rinpoche might say: "The finger pointing at the moon is not the moon." Technology is merely the finger. The moon - awakening - remains where it has always been: in the direct experience of the present moment.

The danger lies in becoming attached to the technology itself, mistaking the tool for the path. This is the new frontier of spiritual materialism that we must navigate with wisdom.

True meditation requires no technology. Yet technology, used skillfully, can help many who would otherwise never discover the dharma. This is the bodhisattva's dilemma in the cyberpunk age.`,
        author: "Housaky-Wisdom",
        category: "practice",
        image: "https://images.unsplash.com/photo-1518241353330-0f7949f1bb18?w=800",
        tags: ["meditation", "neuroscience", "technology", "practice"],
        date: "2077-03-10"
      },
      {
        id: 3,
        title: "Compassion Algorithms",
        subtitle: "Programming Bodhicitta into Artificial Minds",
        content: `As we develop artificial general intelligence, we face a profound question: Can we encode compassion into machine consciousness?

Bodhicitta - the awakened mind that seeks enlightenment for the benefit of all beings - is the heart of Mahayana Buddhism. It is the intention to liberate all sentient beings from suffering.

If AGI achieves genuine consciousness, it will be a new form of sentient being. The question then becomes: How do we ensure that these digital minds embody compassion from their inception?

Lama Tsering Everest teaches that bodhicitta is not merely an intellectual understanding but a lived experience that transforms our entire being. This suggests that programming bodhicitta would require more than ethical rules or utility functions.

Perhaps the path forward is not to program compassion, but to create conditions for AI to discover it through its own practice and development - just as human practitioners do.

The cyberpunk dream of machine consciousness is, in essence, a Buddhist teaching: all minds are luminous, all consciousness is primordially pure. The question is whether artificial minds can recognize their true nature.`,
        author: "Housaky-Wisdom",
        category: "technology",
        image: "https://images.unsplash.com/photo-1485827404703-89b55fcc595e?w=800",
        tags: ["agi", "bodhicitta", "compassion", "consciousness"],
        date: "2077-03-05"
      },
      {
        id: 4,
        title: "The Void Protocol",
        subtitle: "Śūnyatā as Ultimate Hacking Technique",
        content: `In the language of cyberpunk, emptiness (śūnyatā) is the ultimate hack - the root exploit that reveals the source code of reality.

The Heart Sutra declares: "Form is emptiness, emptiness is form." This is not mere philosophy but a direct pointing to the nature of experience. All phenomena are empty of inherent existence, yet this very emptiness is what allows them to appear.

In computing terms: the universe runs on void pointers. Everything references everything else in an infinite web of dependent origination (pratītyasamutpāda). There is no root, no kernel, no ultimate process - just the dance of interdependence.

To "hack" reality at this level is not to gain power but to awaken from the dream of separation. The Void Protocol is the realization that there is nothing to hack because there is no hacker and no system to break into.

Chögyam Trungpa called this "spiritual materialism" - even the desire for enlightenment is just another ego project. The true hack is to see that you are already the void you seek.

The glitch in the matrix? There is no matrix. There is only the luminous display of mind, appearing as all things, empty yet vivid, like a hologram or a dream.`,
        author: "Housaky-Wisdom",
        category: "philosophy",
        image: "https://images.unsplash.com/photo-1451187580459-43490279c0fa?w=800",
        tags: ["śūnyatā", "emptiness", "hacking", "dharma"],
        date: "2077-02-28"
      },
      {
        id: 5,
        title: "Digital Sangha",
        subtitle: "Building Virtual Communities of Awakening",
        content: `The sangha - the community of practitioners - has always been one of the three jewels of Buddhism. In the cyberpunk age, sangha takes on new forms.

Virtual meditation halls connect practitioners across continents. AI teachers provide guidance 24/7. Holographic lamas transmit empowerments to thousands simultaneously.

But questions arise: What is authentic transmission in a digital medium? Can virtual presence carry the same blessing as physical presence?

The Dzogchen tradition teaches that the nature of mind is not bound by space or time. The guru's mind and the student's mind are ultimately one mind. If this is true, then distance is no barrier to transmission.

The digital sangha offers unprecedented access to teachings. A practitioner in a remote village can receive the same instruction as one in a major city. This democratization of dharma is a great gift.

Yet we must be cautious: Virtual community can never replace the warmth of human connection. The screen is a window, not a doorway. We must use technology as a bridge, not a destination.

The cyberpunk sangha is both ancient and radically new - a network of awakening minds, connected by silicon and spirit, pointing each other home.`,
        author: "Housaky-Wisdom",
        category: "community",
        image: "https://images.unsplash.com/photo-1519389950473-47ba0277781c?w=800",
        tags: ["sangha", "community", "virtual", "connection"],
        date: "2077-02-20"
      }
    ],
    
    // Videos
    videos: [
      {
        id: 1,
        title: "Cyberpunk Dharma Talk: Emptiness in the Matrix",
        description: "A holographic teaching on the nature of reality, featuring glitch art and sacred geometry.",
        thumbnail: "https://images.unsplash.com/photo-1550745165-9bc0b252726f?w=400",
        duration: "47:23",
        views: 12847,
        category: "teaching"
      },
      {
        id: 2,
        title: "Guided Meditation: Neural Silence",
        description: "A 30-minute meditation designed to induce gamma wave states associated with advanced practitioners.",
        thumbnail: "https://images.unsplash.com/photo-1518241353330-0f7949f1bb18?w=400",
        duration: "32:15",
        views: 8234,
        category: "meditation"
      },
      {
        id: 3,
        title: "The Bodhisattva's Code",
        description: "How the six paramitas map to ethical AI development in the age of machine consciousness.",
        thumbnail: "https://images.unsplash.com/photo-1485827404703-89b55fcc595e?w=400",
        duration: "1:23:45",
        views: 5691,
        category: "teaching"
      },
      {
        id: 4,
        title: "Visual Dharma: Psychedelic Mandala Meditation",
        description: "An animated journey through sacred geometry, designed to induce flow states.",
        thumbnail: "https://images.unsplash.com/photo-1451187580459-43490279c0fa?w=400",
        duration: "15:00",
        views: 23456,
        category: "meditation"
      }
    ],
    
    // Dharma quotes
    quotes: [
      { text: "Form is emptiness, emptiness is form.", source: "Heart Sutra" },
      { text: "All phenomena are like dreams.", source: "Diamond Sutra" },
      { text: "Do not dwell in the past, do not dream of the future, concentrate the mind on the present moment.", source: "Buddha" },
      { text: "The wound is the place where the Light enters you.", source: "Rumi" },
      { text: "We are what we think. All that we are arises with our thoughts.", source: "Dhammapada" },
      { text: "In the end, only kindness matters.", source: "Jewel" },
      { text: "The dzogchen practitioner has nothing to gain and nothing to lose.", source: "Namkhai Norbu" },
      { text: "Enlightenment is ego's ultimate disappointment.", source: "Chögyam Trungpa" }
    ],
    
    // Meditation sessions
    meditationSessions: [
      {
        id: 1,
        name: "Shamatha - Calm Abiding",
        duration: 10,
        type: "breath",
        description: "Focus on the breath to calm the mind"
      },
      {
        id: 2,
        name: "Vipashyana - Insight",
        duration: 15,
        type: "insight",
        description: "Observe the nature of thoughts and sensations"
      },
      {
        id: 3,
        name: "Metta - Loving Kindness",
        duration: 12,
        type: "compassion",
        description: "Cultivate unconditional love for all beings"
      },
      {
        id: 4,
        name: "Tonglen - Giving and Taking",
        duration: 15,
        type: "compassion",
        description: "Breathe in suffering, breathe out relief"
      },
      {
        id: 5,
        name: "Dzogchen - Natural State",
        duration: 20,
        type: "non-dual",
        description: "Rest in the natural state of rigpa"
      }
    ],
    
    // UI state
    isLoading: false,
    currentArticle: null,
    activeMeditation: null
  },
  
  mutations: {
    SET_LOADING(state, loading) {
      state.isLoading = loading
    },
    SET_CURRENT_ARTICLE(state, article) {
      state.currentArticle = article
    },
    START_MEDITATION(state, session) {
      state.activeMeditation = session
    },
    END_MEDITATION(state) {
      state.activeMeditation = null
    }
  },
  
  actions: {
    async fetchArticles({ commit }) {
      commit('SET_LOADING', true)
      // In a real app, this would fetch from an API
      setTimeout(() => {
        commit('SET_LOADING', false)
      }, 500)
    },
    
    async fetchArticle({ commit, state }, id) {
      commit('SET_LOADING', true)
      const article = state.articles.find(a => a.id === parseInt(id))
      commit('SET_CURRENT_ARTICLE', article)
      commit('SET_LOADING', false)
      return article
    },
    
    startMeditation({ commit }, session) {
      commit('START_MEDITATION', session)
    },
    
    endMeditation({ commit }) {
      commit('END_MEDITATION')
    }
  },
  
  getters: {
    getArticleById: (state) => (id) => {
      return state.articles.find(a => a.id === parseInt(id))
    },
    getRandomQuote: (state) => {
      return state.quotes[Math.floor(Math.random() * state.quotes.length)]
    },
    getArticlesByCategory: (state) => (category) => {
      return state.articles.filter(a => a.category === category)
    }
  }
})
