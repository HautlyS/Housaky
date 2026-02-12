#!/bin/bash
# Comprehensive test, fix, and improvement script for Housaky AGI

set -e

echo "🔬 Housaky AGI - Comprehensive Test & Improvement Suite"
echo "========================================================"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0

check() {
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    echo -n "  [$TOTAL_CHECKS] $1... "
}

pass() {
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
    echo -e "${GREEN}✓${NC}"
}

fail() {
    FAILED_CHECKS=$((FAILED_CHECKS + 1))
    echo -e "${RED}✗${NC}"
    if [ -n "$1" ]; then
        echo "      Error: $1"
    fi
}

warn() {
    echo -e "${YELLOW}⚠${NC}"
    if [ -n "$1" ]; then
        echo "      Warning: $1"
    fi
}

# 1. Code Quality Checks
echo "📋 Phase 1: Code Quality Checks"
echo "--------------------------------"

check "Rust toolchain installed"
if command -v cargo &> /dev/null; then
    pass
else
    fail "Cargo not found"
    exit 1
fi

check "Project compiles (debug)"
if cargo build 2>&1 | grep -q "Finished"; then
    pass
else
    fail "Debug build failed"
fi

check "Project compiles (release)"
if cargo build --release 2>&1 | grep -q "Finished"; then
    pass
else
    fail "Release build failed"
fi

check "No compiler warnings"
BUILD_OUTPUT=$(cargo build --release 2>&1)
if echo "$BUILD_OUTPUT" | grep -q "warning:"; then
    WARNINGS=$(echo "$BUILD_OUTPUT" | grep -c "warning:")
    warn "$WARNINGS warnings found"
else
    pass
fi

check "Clippy lints pass"
CLIPPY_OUTPUT=$(cargo clippy --release 2>&1)
if echo "$CLIPPY_OUTPUT" | grep -q "warning:"; then
    CLIPPY_WARNINGS=$(echo "$CLIPPY_OUTPUT" | grep -c "warning:")
    warn "$CLIPPY_WARNINGS clippy warnings"
else
    pass
fi

check "Code formatting (rustfmt)"
if cargo fmt -- --check &> /dev/null; then
    pass
else
    warn "Code needs formatting (run: cargo fmt)"
fi

# 2. Unit Tests
echo ""
echo "🧪 Phase 2: Unit Tests"
echo "----------------------"

check "All unit tests pass"
TEST_OUTPUT=$(cargo test --release 2>&1)
if echo "$TEST_OUTPUT" | grep -q "test result: ok"; then
    PASSED=$(echo "$TEST_OUTPUT" | grep "test result:" | grep -oP '\d+(?= passed)')
    pass
    echo "      Passed: $PASSED tests"
else
    fail "Some tests failed"
fi

check "No ignored tests"
IGNORED=$(echo "$TEST_OUTPUT" | grep "test result:" | grep -oP '\d+(?= ignored)' || echo "0")
if [ "$IGNORED" -eq 0 ]; then
    pass
else
    warn "$IGNORED tests ignored"
fi

# 3. Integration Tests
echo ""
echo "🔗 Phase 3: Integration Tests"
echo "------------------------------"

check "Binary executable exists"
if [ -f "target/release/housaky" ]; then
    pass
else
    fail "Binary not found"
fi

check "Binary runs with --help"
if timeout 5 ./target/release/housaky --help &> /dev/null; then
    pass
else
    fail "Binary doesn't respond to --help"
fi

check "Binary runs with --version"
if timeout 5 ./target/release/housaky --version &> /dev/null; then
    pass
else
    fail "Binary doesn't respond to --version"
fi

# 4. Feature Completeness
echo ""
echo "✨ Phase 4: Feature Completeness"
echo "--------------------------------"

check "Quantum state module"
if grep -q "pub struct QuantumInspiredState" src/quantum_state.rs; then
    pass
else
    fail "QuantumInspiredState not found"
fi

check "Federated node module"
if grep -q "pub struct FederatedNode" src/federated_node.rs; then
    pass
else
    fail "FederatedNode not found"
fi

check "Photon detector module"
if grep -q "pub struct PhotonDetector" src/photon_detector.rs; then
    pass
else
    fail "PhotonDetector not found"
fi

check "All features in Cargo.toml"
FEATURES=("full-crypto" "camera" "metrics" "web")
for feature in "${FEATURES[@]}"; do
    if grep -q "$feature" Cargo.toml; then
        pass
    else
        fail "Feature $feature not found"
    fi
done

# 5. Documentation
echo ""
echo "📚 Phase 5: Documentation"
echo "-------------------------"

check "README.md exists"
if [ -f "README.md" ]; then
    pass
else
    fail "README.md not found"
fi

check "README has usage instructions"
if grep -q "Usage" README.md; then
    pass
else
    warn "Usage section not found in README"
fi

check "Inline documentation"
DOC_LINES=$(grep -r "///" src/ | wc -l)
if [ "$DOC_LINES" -gt 50 ]; then
    pass
    echo "      Found: $DOC_LINES doc comments"
else
    warn "Only $DOC_LINES doc comments found"
fi

# 6. Performance Tests
echo ""
echo "⚡ Phase 6: Performance Tests"
echo "-----------------------------"

check "Binary size reasonable"
BINARY_SIZE=$(stat -c%s "target/release/housaky" 2>/dev/null || stat -f%z "target/release/housaky" 2>/dev/null)
BINARY_SIZE_MB=$((BINARY_SIZE / 1024 / 1024))
if [ "$BINARY_SIZE_MB" -lt 100 ]; then
    pass
    echo "      Size: ${BINARY_SIZE_MB}MB"
else
    warn "Binary is ${BINARY_SIZE_MB}MB (large)"
fi

check "Compilation time acceptable"
START_TIME=$(date +%s)
cargo build --release &> /dev/null
END_TIME=$(date +%s)
COMPILE_TIME=$((END_TIME - START_TIME))
if [ "$COMPILE_TIME" -lt 60 ]; then
    pass
    echo "      Time: ${COMPILE_TIME}s"
else
    warn "Compilation took ${COMPILE_TIME}s"
fi

# 7. Security Checks
echo ""
echo "🔒 Phase 7: Security Checks"
echo "---------------------------"

check "No hardcoded secrets"
if grep -r "password\|secret\|api_key" src/ --include="*.rs" | grep -v "//"; then
    warn "Potential hardcoded secrets found"
else
    pass
fi

check "Cryptography dependencies"
if grep -q "ed25519-dalek\|blake3" Cargo.toml; then
    pass
else
    fail "Crypto dependencies missing"
fi

check "No unsafe code blocks"
UNSAFE_COUNT=$(grep -r "unsafe" src/ --include="*.rs" | wc -l)
if [ "$UNSAFE_COUNT" -eq 0 ]; then
    pass
else
    warn "$UNSAFE_COUNT unsafe blocks found"
fi

# 8. Deployment Readiness
echo ""
echo "🚀 Phase 8: Deployment Readiness"
echo "---------------------------------"

check "Deploy script exists"
if [ -f "deploy.sh" ]; then
    pass
else
    fail "deploy.sh not found"
fi

check "Deploy script is executable"
if [ -x "deploy.sh" ]; then
    pass
else
    warn "deploy.sh not executable (run: chmod +x deploy.sh)"
fi

check "Docker support"
if [ -f "Dockerfile" ] || grep -q "Dockerfile" deploy.sh; then
    pass
else
    warn "No Dockerfile found"
fi

# 9. Auto-fix Issues
echo ""
echo "🔧 Phase 9: Auto-fix Issues"
echo "---------------------------"

check "Running cargo fix"
if cargo fix --allow-dirty --allow-staged &> /dev/null; then
    pass
else
    warn "Some issues couldn't be auto-fixed"
fi

check "Running cargo fmt"
if cargo fmt; then
    pass
else
    warn "Formatting failed"
fi

# 10. Final Validation
echo ""
echo "✅ Phase 10: Final Validation"
echo "-----------------------------"

check "Clean rebuild"
cargo clean &> /dev/null
if cargo build --release 2>&1 | grep -q "Finished"; then
    pass
else
    fail "Clean rebuild failed"
fi

check "All tests pass after fixes"
if cargo test --release 2>&1 | grep -q "test result: ok"; then
    pass
else
    fail "Tests failed after fixes"
fi

# Summary
echo ""
echo "════════════════════════════════════════════════════════════"
echo "                    SUMMARY REPORT"
echo "════════════════════════════════════════════════════════════"
echo ""
echo "Total Checks:  $TOTAL_CHECKS"
echo -e "Passed:        ${GREEN}$PASSED_CHECKS${NC}"
echo -e "Failed:        ${RED}$FAILED_CHECKS${NC}"
echo -e "Warnings:      ${YELLOW}$((TOTAL_CHECKS - PASSED_CHECKS - FAILED_CHECKS))${NC}"
echo ""

PERCENTAGE=$((PASSED_CHECKS * 100 / TOTAL_CHECKS))
echo "Success Rate:  $PERCENTAGE%"
echo ""

if [ "$FAILED_CHECKS" -eq 0 ]; then
    echo -e "${GREEN}🎉 ALL CRITICAL CHECKS PASSED!${NC}"
    echo ""
    echo "✅ Project is 100% ready for deployment"
    echo ""
    echo "Next steps:"
    echo "  1. Deploy: ./deploy.sh"
    echo "  2. Monitor: ./monitor.sh"
    echo "  3. Test API: curl http://localhost:8080/health"
    echo ""
    exit 0
else
    echo -e "${RED}❌ $FAILED_CHECKS CRITICAL ISSUES FOUND${NC}"
    echo ""
    echo "Please fix the issues above before deployment."
    echo ""
    exit 1
fi
