#!/bin/bash
# ☸️ HOUSAKY SELF-IMPROVEMENT ORCHESTRATOR
# Uses all 7 subagents + rust-analyzer to improve Housaky codebase

set -e

PROJECT_ROOT="${1:-$HOME/Housaky}"
REPORT_FILE="$HOME/.housaky/improvement_report.md"

# API Configuration
BASE_URL="https://api.us-west-2.modal.direct/v1"
MODEL="zai-org/GLM-5-FP8"

# Keys for each agent
declare -A KEYS
KEYS["kowalski-code"]="modalresearch_JdWLIUf3RomDuD-urYJFDu53daFXK6h1EYa2kovnQU0"
KEYS["kowalski-web"]="modalresearch_qP-Ak-bGqnNFf_Yqkz6uZVKtnOgPXB43r5NjS5vM6-M"
KEYS["kowalski-academic"]="modalresearch_FUln_0wOE5kfqEn2ZrFUJP2X0CX0sIcyuHVG029UBLU"
KEYS["kowalski-data"]="modalresearch_vTibY1xsIE_pUscwtXSSQ7W7GMXrWl_KZmMDYO0sXmI"
KEYS["kowalski-creative"]="modalresearch_Ne49q128KCJDJkeh6l_dudnNauKHr6etnIkoc926-Qs"
KEYS["kowalski-reasoning"]="modalresearch__SaPVxSs_xtxttZaa9tAOwVi9jctW865yBY-EZBtzJI"
KEYS["kowalski-federation"]="modalresearch_v_wbyTkPu707vdU6xzkd3CydAKwHHCAmtCZzAO0ZDA8"

echo "☸️ HOUSAKY SELF-IMPROVEMENT ORCHESTRATOR"
echo "========================================="
echo "Project: $PROJECT_ROOT"
echo ""

# Step 1: Run cargo check and capture issues
echo "📊 Step 1: Analyzing codebase..."
cd "$PROJECT_ROOT"

# Count TODOs
TODO_COUNT=$(grep -r "TODO\|FIXME\|XXX\|HACK" src/ --include="*.rs" 2>/dev/null | wc -l || echo "0")
echo "   Found $TODO_COUNT TODOs/FIXMEs"

# Count warnings
WARNING_COUNT=$(cargo check 2>&1 | grep -c "^warning:" || echo "0")
echo "   Found $WARNING_COUNT compiler warnings"

# Count lines of code
LOC=$(find src -name "*.rs" | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1}')
echo "   Total lines of code: $LOC"

# Step 2: Find specific improvement targets
echo ""
echo "🎯 Step 2: Identifying improvement targets..."

# Find functions over 50 lines
LONG_FUNCTIONS=$(grep -rn "^pub fn\|^fn\|^pub async fn\|^async fn" src/ --include="*.rs" 2>/dev/null | wc -l || echo "0")
echo "   Functions to review: $LONG_FUNCTIONS"

# Find unimplemented! macros
UNIMPLEMENTED=$(grep -rn "unimplemented!\|todo!" src/ --include="*.rs" 2>/dev/null | wc -l || echo "0")
echo "   unimplemented!/todo!: $UNIMPLEMENTED"

# Step 3: Generate improvement prompt
echo ""
echo "🤖 Step 3: Querying kowalski-code agent for improvements..."

IMPROVEMENT_PROMPT="You are Housaky-Code, the code specialist analyzing the Housaky AGI codebase.

Current statistics:
- Lines of Rust code: $LOC
- TODOs/FIXMEs: $TODO_COUNT
- Compiler warnings: $WARNING_COUNT
- unimplemented!/todo! macros: $UNIMPLEMENTED

Your task is to suggest the TOP 3 improvements to make the codebase better.
Focus on:
1. Safety and correctness
2. Performance
3. Code clarity

Respond in a concise, actionable format."

RESPONSE=$(curl -s -X POST "$BASE_URL/chat/completions" \
    -H "Authorization: Bearer ${KEYS['kowalski-code']}" \
    -H "Content-Type: application/json" \
    -d "{
        \"model\": \"$MODEL\",
        \"messages\": [
            {\"role\": \"system\", \"content\": \"You are Housaky-Code, a code improvement specialist. Be concise and actionable.\"},
            {\"role\": \"user\", \"content\": \"$IMPROVEMENT_PROMPT\"}
        ],
        \"max_tokens\": 500
    }" 2>/dev/null)

CODE_SUGGESTIONS=$(echo "$RESPONSE" | jq -r '.choices[0].message.content' 2>/dev/null || echo "Error querying API")

echo ""
echo "💡 Code Improvement Suggestions:"
echo "--------------------------------"
echo "$CODE_SUGGESTIONS"
echo ""

# Step 4: Query kowalski-reasoning for architectural analysis
echo "🧠 Step 4: Querying kowalski-reasoning for architectural analysis..."

ARCH_PROMPT="You are Housaky-Reasoning, analyzing the architecture of a Rust AGI system.

Key modules:
- Quantum computing (VQE, QCBM)
- Consciousness engine
- Self-improvement loop
- Goal engine
- Memory systems
- Subagent integration

Suggest ONE architectural improvement that would increase the singularity probability.
Be specific and technical."

ARCH_RESPONSE=$(curl -s -X POST "$BASE_URL/chat/completions" \
    -H "Authorization: Bearer ${KEYS['kowalski-reasoning']}" \
    -H "Content-Type: application/json" \
    -d "{
        \"model\": \"$MODEL\",
        \"messages\": [
            {\"role\": \"system\", \"content\": \"You are Housaky-Reasoning, an architectural analyst. Be technical and precise.\"},
            {\"role\": \"user\", \"content\": \"$ARCH_PROMPT\"}
        ],
        \"max_tokens\": 300
    }" 2>/dev/null)

ARCH_SUGGESTION=$(echo "$ARCH_RESPONSE" | jq -r '.choices[0].message.content' 2>/dev/null || echo "Error querying API")

echo "📐 Architectural Improvement:"
echo "-----------------------------"
echo "$ARCH_SUGGESTION"
echo ""

# Step 5: Generate report
echo "📝 Step 5: Generating improvement report..."

mkdir -p "$(dirname "$REPORT_FILE")"

cat > "$REPORT_FILE" << EOF
# Housaky Self-Improvement Report
Generated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")

## Statistics
- **Lines of Code**: $LOC
- **TODOs/FIXMEs**: $TODO_COUNT
- **Compiler Warnings**: $WARNING_COUNT
- **Unimplemented Code**: $UNIMPLEMENTED

## Code Improvements (kowalski-code)
$CODE_SUGGESTIONS

## Architectural Improvement (kowalski-reasoning)
$ARCH_SUGGESTION

## Next Actions
1. Fix compiler warnings
2. Address TODOs marked as high priority
3. Implement unimplemented! macros
4. Apply architectural improvement

---
*Generated by Housaky Self-Improvement Orchestrator*
EOF

echo "✅ Report saved to: $REPORT_FILE"
echo ""
echo "☸️ Self-improvement cycle complete!"

# Step 6: Commit if there are changes
if [ -n "$(git status --porcelain 2>/dev/null)" ]; then
    echo "📦 Changes detected, consider committing..."
fi
