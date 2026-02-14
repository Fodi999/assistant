#!/bin/bash

# Script to fix migration version conflict in production database
# This removes the old conflicting migration entry from _sqlx_migrations table

echo "🔧 Fixing migration version conflict..."
echo ""
echo "This will:"
echo "1. Remove the duplicate migration entry (20240115000001 - add user activity tracking)"
echo "2. Allow the new migration (20240121000001) to run successfully"
echo ""

# You need to set your DATABASE_URL environment variable or pass it directly
# Example: export DATABASE_URL="postgresql://user:pass@host/db"

if [ -z "$DATABASE_URL" ]; then
    echo "❌ ERROR: DATABASE_URL environment variable is not set"
    echo ""
    echo "Please set it and try again:"
    echo "export DATABASE_URL='your-neon-database-url'"
    exit 1
fi

echo "Executing SQL fix..."
psql "$DATABASE_URL" << 'EOF'
-- Remove the conflicting migration entry
DELETE FROM _sqlx_migrations 
WHERE version = 20240115000001 
  AND description = 'add user activity tracking';

-- Show current migrations
SELECT version, description, installed_on, success 
FROM _sqlx_migrations 
ORDER BY version DESC 
LIMIT 10;
EOF

echo ""
echo "✅ Migration conflict fixed!"
echo ""
echo "Now Koyeb should be able to deploy successfully."
echo "The new migration (20240121000001) will be applied on next deploy."
