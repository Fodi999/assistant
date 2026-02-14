-- Manual fix for migration conflict
-- Run this SQL directly in your Neon database console

-- Step 1: Remove the conflicting migration record
DELETE FROM _sqlx_migrations 
WHERE version = 20240115000001 
  AND description = 'add user activity tracking';

-- Step 2: Check if columns already exist
SELECT column_name, data_type, is_nullable, column_default
FROM information_schema.columns 
WHERE table_name = 'users' 
  AND column_name IN ('login_count', 'last_login_at')
ORDER BY column_name;

-- Step 3: If columns don't exist, create them manually
-- (Run this only if step 2 shows no results)
ALTER TABLE users 
ADD COLUMN IF NOT EXISTS login_count INTEGER NOT NULL DEFAULT 0,
ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMPTZ;

-- Step 4: Create indexes
CREATE INDEX IF NOT EXISTS idx_users_last_login ON users(last_login_at DESC NULLS LAST);
CREATE INDEX IF NOT EXISTS idx_users_login_count ON users(login_count DESC);

-- Step 5: Verify the setup
SELECT 
    COUNT(*) as total_users,
    SUM(CASE WHEN login_count > 0 THEN 1 ELSE 0 END) as users_with_logins,
    MAX(last_login_at) as most_recent_login
FROM users;

-- Step 6: Check migrations table
SELECT version, description, installed_on, success 
FROM _sqlx_migrations 
ORDER BY version DESC 
LIMIT 15;

COMMENT ON COLUMN users.login_count IS 'Number of times user has logged in';
COMMENT ON COLUMN users.last_login_at IS 'Timestamp of user''s last successful login';
