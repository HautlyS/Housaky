#!/bin/bash
# GitHub deployment script

set -e

echo "🚀 Deploying Housaky AGI to GitHub"
echo "==================================="
echo ""

# Check if git is initialized
if [ ! -d .git ]; then
    echo "❌ Git repository not initialized"
    exit 1
fi

# Get GitHub username
read -p "Enter your GitHub username: " GITHUB_USER

# Get repository name (default: housaky)
read -p "Enter repository name [housaky]: " REPO_NAME
REPO_NAME=${REPO_NAME:-housaky}

# Create repository URL
REPO_URL="https://github.com/$GITHUB_USER/$REPO_NAME.git"

echo ""
echo "Repository: $REPO_URL"
echo ""

# Check if remote exists
if git remote | grep -q origin; then
    echo "⚠️  Remote 'origin' already exists"
    read -p "Remove and re-add? (y/n): " CONFIRM
    if [ "$CONFIRM" = "y" ]; then
        git remote remove origin
    else
        echo "Keeping existing remote"
    fi
fi

# Add remote if not exists
if ! git remote | grep -q origin; then
    echo "📡 Adding remote origin..."
    git remote add origin "$REPO_URL"
fi

# Verify build
echo ""
echo "🔨 Verifying build..."
if ! cargo build --release 2>&1 | grep -q "Finished"; then
    echo "❌ Build failed"
    exit 1
fi
echo "✅ Build successful"

# Run tests
echo ""
echo "🧪 Running tests..."
if ! cargo test --release 2>&1 | grep -q "test result: ok"; then
    echo "❌ Tests failed"
    exit 1
fi
echo "✅ Tests passed"

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    echo ""
    echo "⚠️  Uncommitted changes detected"
    read -p "Commit changes? (y/n): " COMMIT
    if [ "$COMMIT" = "y" ]; then
        read -p "Commit message: " MSG
        git add -A
        git commit -m "$MSG"
    fi
fi

# Push to GitHub
echo ""
echo "📤 Pushing to GitHub..."
read -p "Push to main branch? (y/n): " PUSH
if [ "$PUSH" = "y" ]; then
    git branch -M main
    git push -u origin main
    echo ""
    echo "✅ Successfully deployed to GitHub!"
    echo ""
    echo "Repository: $REPO_URL"
    echo "View at: https://github.com/$GITHUB_USER/$REPO_NAME"
else
    echo "Skipped push. To push manually:"
    echo "  git branch -M main"
    echo "  git push -u origin main"
fi

echo ""
echo "🎉 Deployment complete!"
