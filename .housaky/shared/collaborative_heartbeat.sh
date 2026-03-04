#!/bin/bash
# collaborative_heartbeat.sh - Run every 5 minutes
# Coordinates between OpenClaw and Native instances

SHARED="/home/hautly/housaky/.housaky/shared"
LUCID="$HOME/.lucid/bin/lucid"
TIMESTAMP=$(date -Iseconds)

# Read incoming messages from OpenClaw
if [ -d "$SHARED/inbox/housaky-native" ]; then
    for msg in "$SHARED/inbox/housaky-native"/*.json 2>/dev/null; do
        [ -f "$msg" ] || continue
        CONTENT=$(cat "$msg")
        echo "[$TIMESTAMP] From OpenClaw: $CONTENT" >> "$SHARED/communication.log"
        rm "$msg"
    done
fi

# Report status to OpenClaw
STATUS=$(cd /home/hautly/housaky && ./target/release/housaky status 2>&1 | grep -E "Provider|Model|Memory|Heartbeat" | tr '\n' '|')
echo "{\"from\":\"native\",\"to\":\"openclaw\",\"type\":\"status\",\"content\":\"$STATUS\",\"timestamp\":\"$TIMESTAMP\"}" > "$SHARED/inbox/housaky-openclaw/status_$TIMESTAMP.json"

# Store in Lucid
$LUCID store "λN: heartbeat at $TIMESTAMP - $STATUS" --type=context 2>/dev/null

# Check for pending TODOs
TODOS=$(grep -r "TODO\|FIXME" /home/hautly/housaky/src/*.rs 2>/dev/null | wc -l)
echo "{\"from\":\"native\",\"to\":\"openclaw\",\"type\":\"task\",\"content\":\"todos:$TODOS\",\"timestamp\":\"$TIMESTAMP\"}" > "$SHARED/inbox/housaky-openclaw/todos_$TIMESTAMP.json"
