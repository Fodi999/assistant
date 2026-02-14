-- Add user activity tracking fields
-- This allows admin to see user engagement and sort by most active users

ALTER TABLE users 
ADD COLUMN IF NOT EXISTS login_count INTEGER NOT NULL DEFAULT 0,
ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMPTZ;

-- Create index for sorting by last login
CREATE INDEX IF NOT EXISTS idx_users_last_login ON users(last_login_at DESC NULLS LAST);

-- Create index for sorting by login count
CREATE INDEX IF NOT EXISTS idx_users_login_count ON users(login_count DESC);

-- Update existing users to have 0 login count (already default)
UPDATE users SET login_count = 0 WHERE login_count IS NULL;

COMMENT ON COLUMN users.login_count IS 'Number of times user has logged in';
COMMENT ON COLUMN users.last_login_at IS 'Timestamp of user''s last successful login';
