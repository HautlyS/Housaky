#!/usr/bin/env python3
"""
UI-UX-Pro-Max Search Script

BM25-style ranking search across design databases.
Provides style, color, typography, and design system recommendations.

Usage:
    python search.py --domain style --query "modern dark"
    python search.py --domain color --query "saas dashboard"
    python search.py --domain typography --query "tech startup"
    python search.py --design-system "fintech dashboard"
    python search.py --stack react --domain style --query "glassmorphism"
"""

import argparse
import csv
import math
import os
import re
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Tuple


@dataclass
class SearchResult:
    """Represents a search result with score and data."""
    score: float
    data: Dict[str, str]
    source: str


class BM25Searcher:
    """BM25-style search implementation for CSV databases."""
    
    def __init__(self, k1: float = 1.5, b: float = 0.75):
        self.k1 = k1
        self.b = b
        self.documents: List[Dict[str, str]] = []
        self.doc_lengths: List[int] = []
        self.avg_doc_length: float = 0.0
        self.doc_term_freqs: List[Counter] = []
        self.doc_freqs: Dict[str, int] = {}
        self.idf: Dict[str, float] = {}
    
    def tokenize(self, text: str) -> List[str]:
        """Tokenize text into lowercase terms."""
        text = text.lower()
        text = re.sub(r'[_\W]+', ' ', text)
        return [t for t in text.split() if t]
    
    def index(self, documents: List[Dict[str, str]]) -> None:
        """Index documents for BM25 search."""
        self.documents = documents
        self.doc_lengths = []
        self.doc_term_freqs = []
        self.doc_freqs = Counter()
        
        all_terms = set()
        
        for doc in documents:
            combined_text = ' '.join(str(v) for v in doc.values())
            tokens = self.tokenize(combined_text)
            self.doc_lengths.append(len(tokens))
            
            term_counts = Counter(tokens)
            self.doc_term_freqs.append(term_counts)
            all_terms.update(tokens)
            
            for term in term_counts:
                self.doc_freqs[term] += 1
        
        self.avg_doc_length = sum(self.doc_lengths) / len(self.doc_lengths) if self.doc_lengths else 1
        
        N = len(documents)
        for term in all_terms:
            df = self.doc_freqs.get(term, 0)
            self.idf[term] = math.log((N - df + 0.5) / (df + 0.5) + 1)
    
    def search(self, query: str, top_k: int = 10) -> List[Tuple[float, Dict[str, str]]]:
        """Search documents using BM25 scoring."""
        query_tokens = self.tokenize(query)
        scores = []
        
        for doc_idx, doc in enumerate(self.documents):
            score = 0.0
            doc_len = self.doc_lengths[doc_idx]
            term_counts = self.doc_term_freqs[doc_idx]
            
            for term in query_tokens:
                if term not in self.idf:
                    continue
                
                tf = term_counts.get(term, 0)
                if tf == 0:
                    continue
                
                idf = self.idf[term]
                
                numerator = tf * (self.k1 + 1)
                denominator = tf + self.k1 * (1 - self.b + self.b * doc_len / self.avg_doc_length)
                score += idf * numerator / denominator
            
            scores.append((score, doc))
        
        scores.sort(key=lambda x: x[0], reverse=True)
        return scores[:top_k]


class DataLoader:
    """Load and cache CSV data files."""
    
    def __init__(self, data_dir: Optional[Path] = None):
        if data_dir is None:
            data_dir = Path(__file__).parent.parent / 'data'
        self.data_dir = Path(data_dir)
        self._cache: Dict[str, List[Dict[str, str]]] = {}
    
    def load_csv(self, filename: str) -> List[Dict[str, str]]:
        """Load CSV file and return list of dictionaries."""
        if filename in self._cache:
            return self._cache[filename]
        
        filepath = self.data_dir / filename
        if not filepath.exists():
            return []
        
        with open(filepath, 'r', encoding='utf-8') as f:
            reader = csv.DictReader(f)
            data = [row for row in reader]
        
        self._cache[filename] = data
        return data
    
    def load_styles(self) -> List[Dict[str, str]]:
        return self.load_csv('styles.csv')
    
    def load_colors(self) -> List[Dict[str, str]]:
        return self.load_csv('colors.csv')
    
    def load_typography(self) -> List[Dict[str, str]]:
        return self.load_csv('typography.csv')


class DesignSystemGenerator:
    """Generate complete design systems from product requirements."""
    
    def __init__(self, data_loader: DataLoader):
        self.loader = data_loader
        self.style_searcher = BM25Searcher()
        self.color_searcher = BM25Searcher()
        self.typo_searcher = BM25Searcher()
        
        self._init_searchers()
    
    def _init_searchers(self):
        styles = self.loader.load_styles()
        colors = self.loader.load_colors()
        typography = self.loader.load_typography()
        
        if styles:
            self.style_searcher.index(styles)
        if colors:
            self.color_searcher.index(colors)
        if typography:
            self.typo_searcher.index(typography)
    
    def generate(self, product_description: str, stack: Optional[str] = None) -> Dict:
        """Generate a complete design system."""
        results = {
            'product': product_description,
            'stack': stack or 'html-tailwind',
            'style': None,
            'colors': None,
            'typography': None,
            'guidelines': []
        }
        
        expanded_query = self._expand_query(product_description)
        
        style_results = self.style_searcher.search(expanded_query, top_k=1)
        if style_results:
            results['style'] = style_results[0][1]
        
        color_results = self.color_searcher.search(product_description, top_k=1)
        if color_results:
            results['colors'] = color_results[0][1]
        
        typo_results = self.typo_searcher.search(expanded_query, top_k=1)
        if typo_results:
            results['typography'] = typo_results[0][1]
        
        results['guidelines'] = self._generate_guidelines(results)
        
        return results
    
    def _expand_query(self, query: str) -> str:
        """Expand query with domain-related terms for better matching."""
        domain_mappings = {
            'fintech': 'professional modern corporate tech',
            'banking': 'professional corporate modern',
            'saas': 'modern tech startup professional',
            'dashboard': 'modern minimal clean professional',
            'healthcare': 'clean soft minimal professional',
            'wellness': 'soft organic natural calm',
            'ecommerce': 'modern clean vibrant shopping',
            'creative': 'modern bold artistic creative',
            'portfolio': 'minimal modern clean elegant',
            'gaming': 'dark modern bold vibrant',
            'crypto': 'dark modern tech innovative',
            'education': 'clean friendly modern',
            'startup': 'modern fresh tech bold',
            'enterprise': 'professional corporate clean',
            'luxury': 'elegant sophisticated minimal',
            'food': 'warm organic natural friendly',
            'travel': 'modern clean vibrant adventurous',
            'fitness': 'bold energetic vibrant modern',
            'music': 'dark modern creative bold',
            'social': 'modern friendly vibrant clean',
        }
        
        query_lower = query.lower()
        expanded = query
        
        for key, expansion in domain_mappings.items():
            if key in query_lower:
                expanded = f"{query} {expansion}"
                break
        
        return expanded
    
    def _generate_guidelines(self, system: Dict) -> List[str]:
        """Generate practical guidelines based on the design system."""
        guidelines = []
        
        if system.get('style'):
            style = system['style']
            guidelines.append(f"Style: Use {style['name']} approach")
            if style.get('effects'):
                guidelines.append(f"Effects: {style['effects']}")
            if style.get('accessibility_notes'):
                guidelines.append(f"Accessibility: {style['accessibility_notes']}")
        
        if system.get('colors'):
            colors = system['colors']
            guidelines.append(f"Primary: {colors.get('primary', 'N/A')}")
            guidelines.append(f"Secondary: {colors.get('secondary', 'N/A')}")
            guidelines.append(f"Accent: {colors.get('accent', 'N/A')}")
        
        if system.get('typography'):
            typo = system['typography']
            guidelines.append(f"Heading Font: {typo.get('heading_font', 'N/A')}")
            guidelines.append(f"Body Font: {typo.get('body_font', 'N/A')}")
        
        return guidelines
    
    def format_output(self, system: Dict, format_type: str = 'markdown') -> str:
        """Format design system for output."""
        if format_type == 'json':
            import json
            return json.dumps(system, indent=2)
        
        lines = [
            f"# Design System: {system['product']}",
            "",
            f"**Target Stack:** {system['stack']}",
            "",
        ]
        
        if system.get('style'):
            style = system['style']
            lines.extend([
                "## Style",
                f"- **Name:** {style['name']}",
                f"- **Keywords:** {style.get('keywords', 'N/A')}",
                f"- **Effects:** {style.get('effects', 'N/A')}",
                f"- **Complexity:** {style.get('complexity', 'N/A')}",
                f"- **Accessibility:** {style.get('accessibility_notes', 'N/A')}",
                "",
            ])
        
        if system.get('colors'):
            colors = system['colors']
            lines.extend([
                "## Color Palette",
                f"- **Primary:** `{colors.get('primary', 'N/A')}`",
                f"- **Secondary:** `{colors.get('secondary', 'N/A')}`",
                f"- **Accent:** `{colors.get('accent', 'N/A')}`",
                f"- **Background:** `{colors.get('background', 'N/A')}`",
                f"- **Text:** `{colors.get('text', 'N/A')}`",
                f"- **Use Case:** {colors.get('use_case', 'N/A')}",
                "",
            ])
        
        if system.get('typography'):
            typo = system['typography']
            lines.extend([
                "## Typography",
                f"- **Pairing:** {typo['name']}",
                f"- **Heading Font:** {typo.get('heading_font', 'N/A')}",
                f"- **Body Font:** {typo.get('body_font', 'N/A')}",
                f"- **Character:** {typo.get('character', 'N/A')}",
                f"- **Google Fonts:** {typo.get('google_fonts_url', 'N/A')}",
                "",
            ])
        
        if system.get('guidelines'):
            lines.extend([
                "## Guidelines",
                *[f"- {g}" for g in system['guidelines']],
            ])
        
        return '\n'.join(lines)


class StackFormatter:
    """Format output for specific tech stacks."""
    
    @staticmethod
    def format_colors_tailwind(colors: Dict) -> str:
        return f"""// tailwind.config.js
module.exports = {{
  theme: {{
    extend: {{
      colors: {{
        primary: '{colors.get("primary", "#3B82F6")}',
        secondary: '{colors.get("secondary", "#10B981")}',
        accent: '{colors.get("accent", "#F59E0B")}',
        background: '{colors.get("background", "#F8FAFC")}',
        text: '{colors.get("text", "#1E293B")}',
      }}
    }}
  }}
}}"""
    
    @staticmethod
    def format_colors_css(colors: Dict) -> str:
        return f""":root {{
  --color-primary: {colors.get("primary", "#3B82F6")};
  --color-secondary: {colors.get("secondary", "#10B981")};
  --color-accent: {colors.get("accent", "#F59E0B")};
  --color-background: {colors.get("background", "#F8FAFC")};
  --color-text: {colors.get("text", "#1E293B")};
}}"""
    
    @staticmethod
    def format_typography_css(typo: Dict) -> str:
        return f"""/* Typography */
@import url('{typo.get("google_fonts_url", "")}');

h1, h2, h3, h4, h5, h6 {{
  font-family: '{typo.get("heading_font", "Inter")}', sans-serif;
}}

body {{
  font-family: '{typo.get("body_font", "Inter")}', sans-serif;
}}"""


def search_domain(query: str, domain: str, data_loader: DataLoader, top_k: int = 5) -> List[SearchResult]:
    """Search a specific domain."""
    searcher = BM25Searcher()
    
    domain_files = {
        'style': 'styles.csv',
        'color': 'colors.csv',
        'typography': 'typography.csv',
    }
    
    if domain not in domain_files:
        return []
    
    data = data_loader.load_csv(domain_files[domain])
    if not data:
        return []
    
    searcher.index(data)
    results = searcher.search(query, top_k=top_k)
    
    return [SearchResult(score=score, data=d, source=domain) for score, d in results]


def format_search_results(results: List[SearchResult], domain: str) -> str:
    """Format search results for output."""
    if not results:
        return f"No results found for domain: {domain}"
    
    lines = [f"## Search Results ({domain})", ""]
    
    for i, result in enumerate(results, 1):
        lines.append(f"### Result {i} (Score: {result.score:.2f})")
        for key, value in result.data.items():
            lines.append(f"- **{key}:** {value}")
        lines.append("")
    
    return '\n'.join(lines)


def main():
    parser = argparse.ArgumentParser(
        description='UI-UX-Pro-Max Search - BM25-powered design database search',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s --domain style --query "modern dark"
  %(prog)s --domain color --query "saas dashboard"
  %(prog)s --domain typography --query "tech startup"
  %(prog)s --design-system "fintech dashboard"
  %(prog)s --design-system "healthcare app" --stack react
        """
    )
    
    parser.add_argument('--domain', '-d', 
                        choices=['style', 'color', 'typography', 'landing', 'chart', 'ux'],
                        help='Domain to search')
    parser.add_argument('--query', '-q',
                        help='Search query')
    parser.add_argument('--design-system', '-s',
                        help='Generate complete design system for product')
    parser.add_argument('--stack',
                        choices=['html-tailwind', 'react', 'nextjs', 'shadcn', 'vue', 'svelte', 'swiftui', 'react-native', 'flutter'],
                        default='html-tailwind',
                        help='Target tech stack for code output')
    parser.add_argument('--top-k', '-k',
                        type=int,
                        default=5,
                        help='Number of results to return (default: 5)')
    parser.add_argument('--format', '-f',
                        choices=['markdown', 'json'],
                        default='markdown',
                        help='Output format')
    parser.add_argument('--data-dir',
                        help='Custom data directory path')
    
    args = parser.parse_args()
    
    data_dir = Path(args.data_dir) if args.data_dir else None
    loader = DataLoader(data_dir)
    
    if args.design_system:
        generator = DesignSystemGenerator(loader)
        system = generator.generate(args.design_system, args.stack)
        print(generator.format_output(system, args.format))
    elif args.domain and args.query:
        results = search_domain(args.query, args.domain, loader, args.top_k)
        if args.format == 'json':
            import json
            output = [{'score': r.score, 'data': r.data} for r in results]
            print(json.dumps(output, indent=2))
        else:
            print(format_search_results(results, args.domain))
    else:
        parser.print_help()
        sys.exit(1)


if __name__ == '__main__':
    main()