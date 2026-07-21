#!/usr/bin/env sh
set -eu
BASE_URL="${BASE_URL:-http://localhost:8080}"

echo "1. Health"
curl --fail --silent "$BASE_URL/api/v1/health"
echo

echo "2. Scenario"
curl --fail --silent "$BASE_URL/api/v1/scenario" | grep -q 'civic-drainage-v1'
echo "Scenario endpoint OK"

echo "3. Create session"
SESSION_JSON=$(curl --fail --silent \
  -H 'Content-Type: application/json' \
  -d '{"citizen_id":"shopkeeper"}' \
  "$BASE_URL/api/v1/sessions")
SESSION_ID=$(printf '%s' "$SESSION_JSON" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
[ -n "$SESSION_ID" ]
echo "Created $SESSION_ID"

echo "4. Take action"
curl --fail --silent \
  -H 'Content-Type: application/json' \
  -d '{"action_id":"file_complaint","client_action_id":"smoke-test-1"}' \
  "$BASE_URL/api/v1/sessions/$SESSION_ID/actions" | grep -q 'outcome_id'
echo "Action endpoint OK"

echo "Smoke test passed"
