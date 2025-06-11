#! /usr/bin/env nix-shell
#! nix-shell -i bash -p curl jq

set -euo pipefail

API_URL="http://127.0.0.1:8080"

echo "# Creating entity"
CREATE_RESP=$(curl -s -X POST "$API_URL/entities" \
    -H "Content-Type: application/json" \
    -d '{"data":{"foo":"bar"}, "owner":"'"$ACCOUNT_ADDRESS"'"}')
echo "Create response: $CREATE_RESP"

ENTITY_ID=$(echo "$CREATE_RESP" | jq -r '.[0].entity_key // .entity_key // empty')

if [ -z "$ENTITY_ID" ]; then
    echo "Failed to extract entity ID"
    exit 1
fi

echo "# Getting entities for owner"
curl -s "$API_URL/entities/$ACCOUNT_ADDRESS" | jq .

echo "# Updating entity"
curl -s -X PUT "$API_URL/entities/$ENTITY_ID" \
    -H "Content-Type: application/json" \
    -d '{"entity_key":"'"$ENTITY_ID"'", "data":{"foo":"baz"}}'

echo "# Deleting entity"
curl -s -X DELETE "$API_URL/entities/$ENTITY_ID"

echo "# Test completed"
