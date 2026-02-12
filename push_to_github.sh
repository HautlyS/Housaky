#!/bin/bash
# Automated GitHub push script

set -e

echo "🚀 Pushing Housaky AGI to GitHub"
echo "================================="
echo ""

# Check if we're in a git repo
if [ ! -d .git ]; then
    echo "❌ Not a git repository"
    exit 1
fi

# Verify build first
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

# Get GitHub username from user
read -p "Enter your GitHub username: " GITHUB_USER
if [ -z "$GITHUB_USER" ]; then
    echo "❌ GitHub username required"
    exit 1
fi

# Repository name
REPO_NAME="housaky"
REPO_URL="https://github.com/$GITHUB_USER/$REPO_NAME.git"

echo ""
echo "Repository: $REPO_URL"
echo ""

# Check if remote exists
if git remote | grep -q origin; then
    echo "⚠️  Remote 'origin' already exists"
    EXISTING_URL=$(git remote get-url origin)
    echo "   Current: $EXISTING_URL"
    read -p "Update to $REPO_URL? (y/n): " UPDATE
    if [ "$UPDATE" = "y" ]; then
        git remote set-url origin "$REPO_URL"
        echo "✅ Remote updated"
    fi
else
    echo "📡 Adding remote origin..."
    git remote add origin "$REPO_URL"
    echo "✅ Remote added"
fi

# Check for uncommitted changes
if ! git diff-index --quiet HEAD -- 2>/dev/null; then
    echo ""
    echo "⚠️  Uncommitted changes detected"
    git status --short
    read -p "Commit all changes? (y/n): " COMMIT
    if [ "$COMMIT" = "y" ]; then
        git add -A
        read -p "Commit message [Update]: " MSG
        MSG=${MSG:-"Update"}
        git commit -m "$MSG"
        echo "✅ Changes committed"
    fi
fi

# Show what will be pushed
echo ""
echo "📦 Ready to push:"
git log --oneline -5
echo ""

# Push to GitHub
read -p "Push to GitHub? (y/n): " PUSH
if [ "$PUSH" = "y" ]; then
    echo ""
    echo "📤 Pushing to GitHub..."
    
    # Rename branch to main if needed
    CURRENT_BRANCH=$(git branch --show-current)
    if [ "$CURRENT_BRANCH" = "master" ]; then
        git branch -M main
        echo "✅ Renamed master → main"
    fi
    
    # Push
    if git push -u origin main; then
        echo ""
        echo "╔══════════════════════════════════════════════════════════╗"
        echo "║                                                          ║"
        echo "║          ✅ SUCCESSFULLY PUSHED TO GITHUB! ✅            ║"
        echo "║                                                          ║"
        echo "╚══════════════════════════════════════════════════════════╝"
        echo ""
        echo "🎉 Repository: https://github.com/$GITHUB_USER/$REPO_NAME"
        echo ""
        echo "Next steps:"
        echo "  1. Visit: https://github.com/$GITHUB_USER/$REPO_NAME"
        echo "  2. Add description: 'Autonomous Self-Improving Distributed Intelligence'"
        echo "  3. Add topics: rust, agi, quantum-computing, distributed-systems"
        echo "  4. Enable: Issues, Discussions, Wiki"
        echo "  5. Watch GitHub Actions run your CI/CD pipeline"
        echo ""
    else
        echo ""
        echo "❌ Push failed"
        echo ""
        echo "Common issues:"
        echo "  1. Repository doesn't exist - Create it at: https://github.com/new"
        echo "  2. Authentication failed - Set up SSH keys or use personal access token"
        echo "  3. Permission denied - Check repository permissions"
        echo ""
        echo "Manual push:"
        echo "  git push -u origin main"
        exit 1
    fi
else
    echo ""
    echo "Push cancelled. To push manually:"
    echo "  git push -u origin main"
fi

echo ""
echo "🎉 Done!"
