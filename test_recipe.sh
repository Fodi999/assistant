#!/bin/bash
TOKEN=$(cat /tmp/tok.txt)

curl -s -X POST http://localhost:8000/api/recipes/v2 \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Пельмени",
    "instructions": "Варить 10 минут",
    "language": "ru",
    "servings": 4,
    "ingredients": [{
      "catalog_ingredient_id": "8238ad5e-f9d2-4edd-8690-9ba68e07a3f8",
      "quantity": 0.3,
      "unit": "kg"
    }]
  }'
