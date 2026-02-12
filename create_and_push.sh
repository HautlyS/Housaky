#!/bin/bash
# Create GitHub repo and push

set -e

echo "🚀 Creating GitHub Repository and Pushing"
echo "=========================================="
echo ""

# Check gh CLI
if ! command -v gh &> /dev/null; then
    echo "❌ GitHub CLI not installed"
    echo ""
    echo "Install with:"
    echo "  Ubuntu/Debian: sudo apt install gh"
    echo "  macOS: brew install gh"
    echo "  Or: https://cli.github.com/"
    echo ""
    echo "After install, authenticate:"
    echo "  gh auth login"
    exit 1
fi

# Check authentication
if ! gh auth status &> /dev/null; then
    echo "❌ Not authenticated with GitHub"
    echo ""
    echo "Run: gh auth login"
    exit 1
fi

# Verify build
echo "🔨 Verifying build..."
if ! cargo build --release 2>&1 | grep -q "Finished"; then
    echo "❌ Build failed"
    exit 1
fi
echo "✅ Build successful"

# Run tests
echo "🧪 Running tests..."
if ! cargo test --release 2>&1 | grep -q "test result: ok"; then
    echo "❌ Tests failed"
    exit 1
fi
echo "✅ Tests passed"

# Create repository
REPO_NAME="housaky-agi"
echo ""
echo "📦 Creating repository: $REPO_NAME"
read -p "Make repository public? (y/n) [y]: " PUBLIC
PUBLIC=${PUBLIC:-y}

if [ "$PUBLIC" = "y" ]; then
    VISIBILITY="--public"
else
    VISIBILITY="--private"
fi

# Create repo
gh repo create "$REPO_NAME" $VISIBILITY \
    --description "Autonomous Self-Improving Distributed Intelligence - Quantum-inspired AGI with federated learning" \
    --source=. \
    --remote=origin \
    --push || {
    echo "⚠️  Repository might already exist, trying to push..."
    git remote add origin "https://github.com/$(gh api user -q .login)/$REPO_NAME.git" 2>/dev/null || true
    git branch -M main
    git push -u origin main
}

echo ""
echo "✅ Repository created and pushed!"
echo ""
echo "🔗 Repository: https://github.com/$(gh api user -q .login)/$REPO_NAME"
echo ""
echo "Next steps:"
echo "  1. View: gh repo view --web"
echo "  2. Watch Actions: gh run list"
echo "  3. Create release: git tag v2.0.0 && git push origin v2.0.0"

