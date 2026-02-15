#!/bin/bash
set -e

TOKEN=$(cat /tmp/tok.txt)

echo "Creating recipe..."
RESPONSE=$(curl -s -X POST http://localhost:8000/api/recipes/v2 \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name":"Блины",
    "instructions":"Жарить 2 минуты с каждой стороны",
    "language":"ru",
    "servings":2,
    "ingredients":[{
      "catalog_ingredient_id":"8238ad5e-f9d2-4edd-8690-9ba68e07a3f8",
      "quantity":0.2,
      "unit":"kg"
    }]
  }')

RECIPE_ID=$(echo "$RESPONSE" | jq -r '.id')
echo "Recipe created: $RECIPE_ID"
echo "Waiting 15 seconds for AI translations..."
sleep 15

echo ""
echo "Translations:"
psql "$(cat .env | grep DATABASE_URL | cut -d'=' -f2-)" -c "SELECT language, name, translated_by FROM recipe_translations WHERE recipe_id = '$RECIPE_ID' ORDER BY language;"
