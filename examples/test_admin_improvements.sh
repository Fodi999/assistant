#!/bin/bash

BASE_URL="https://ministerial-yetta-fodi999-c58d8823.koyeb.app"
ADMIN_TOKEN="eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJhZG1pbkBmb2RpLmFwcCIsInRlbmFudF9pZCI6IjAwMDAwMDAwLTAwMDAtMDAwMC0wMDAwLTAwMDAwMDAwMDAwMCIsImlzcyI6InJlc3RhdXJhbnQtYmFja2VuZCIsImlhdCI6MTczOTM0Mjc2MCwiZXhwIjoxNzQxOTM0NzYwfQ.6pf9sFZ_c-2Y7L4gImNjXevNE4VGYd7qTUNkdkGBizI"

echo "=== Test 1: Empty name_en validation ==="
curl -s -X POST "$BASE_URL/api/admin/catalog/products" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name_en":"  ","category_id":"5a841ce0-2ea5-4230-a1f7-011fa445afdc","price":5.00,"unit":"kilogram"}' | jq

echo ""
echo "=== Test 2: Duplicate product ==="
curl -s -X POST "$BASE_URL/api/admin/catalog/products" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name_en":"Tomato","category_id":"5a841ce0-2ea5-4230-a1f7-011fa445afdc","price":5.00,"unit":"kilogram"}' | jq

echo ""
echo "=== Test 3: Empty translations fallback ==="
curl -s -X POST "$BASE_URL/api/admin/catalog/products" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name_en":"Test Product","name_pl":"","name_uk":"","name_ru":"","category_id":"5a841ce0-2ea5-4230-a1f7-011fa445afdc","price":12.99,"unit":"piece"}' | jq '.name_en, .name_pl, .name_uk, .name_ru'

echo ""
echo "=== Test 4: Case-insensitive duplicate ==="
curl -s -X POST "$BASE_URL/api/admin/catalog/products" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name_en":"TOMATO","category_id":"5a841ce0-2ea5-4230-a1f7-011fa445afdc","price":5.00,"unit":"kilogram"}' | jq
