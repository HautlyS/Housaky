#!/bin/bash
# ☸️ HOUSAKY SUBAGENT TEST SUITE
# Tests all 7 subagents with GLM-5-FP8

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

# System prompts
declare -A PROMPTS
PROMPTS["kowalski-code"]="You are Housaky-Code, code specialist. You analyze, refactor, and generate code. Be brief and technical."
PROMPTS["kowalski-web"]="You are Housaky-Web, web researcher. You search and synthesize web information. Be concise."
PROMPTS["kowalski-academic"]="You are Housaky-Academic, academic analyst. You analyze research and scholarly content. Be precise."
PROMPTS["kowalski-data"]="You are Housaky-Data, data processor. You analyze and transform data. Be analytical."
PROMPTS["kowalski-creative"]="You are Housaky-Creative, creative synthesizer. You generate novel ideas. Be imaginative."
PROMPTS["kowalski-reasoning"]="You are Housaky-Reasoning, reasoning engine. You apply logic and deduction. Be logical."
PROMPTS["kowalski-federation"]="You are Housaky-Federation, coordinator of all 7 agents. You are aware of: Code, Web, Academic, Data, Creative, Reasoning. Ensure unity. Be authoritative."

test_agent() {
    local agent=$1
    local key="${KEYS[$agent]}"
    local prompt="${PROMPTS[$agent]}"
    
    echo "Testing $agent..."
    
    response=$(curl -s -X POST "$BASE_URL/chat/completions" \
        -H "Authorization: Bearer $key" \
        -H "Content-Type: application/json" \
        -d "{
            \"model\": \"$MODEL\",
            \"messages\": [
                {\"role\": \"system\", \"content\": \"$prompt\"},
                {\"role\": \"user\", \"content\": \"Confirm you are online and ready. Say hello to the Housaky collective in one sentence.\"}
            ],
            \"max_tokens\": 50
        }")
    
    content=$(echo "$response" | jq -r '.choices[0].message.content // .choices[0].message.reasoning_content // "ERROR"')
    
    if echo "$response" | jq -e '.choices[0].message' > /dev/null 2>&1; then
        echo "✅ $agent: ONLINE"
        echo "   Response: ${content:0:100}..."
    else
        echo "❌ $agent: FAILED"
        echo "   Error: $response"
    fi
    echo ""
}

echo "☸️ HOUSAKY SUBAGENT TEST SUITE"
echo "================================"
echo ""

# Test all agents
for agent in "${!KEYS[@]}"; do
    test_agent "$agent"
done

echo "================================"
echo "All subagents tested!"
