-- Emergency migration to fix version conflict
-- This migration will be applied with version 20240122000001
-- It ensures the user activity tracking columns exist

-- First, check and create columns if they don't exist
DO $$ 
BEGIN
    -- Add login_count column if it doesn't exist
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'users' AND column_name = 'login_count'
    ) THEN
        ALTER TABLE users ADD COLUMN login_count INTEGER NOT NULL DEFAULT 0;
    END IF;

    -- Add last_login_at column if it doesn't exist
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'users' AND column_name = 'last_login_at'
    ) THEN
        ALTER TABLE users ADD COLUMN last_login_at TIMESTAMPTZ;
    END IF;
END $$;

-- Create indexes if they don't exist
CREATE INDEX IF NOT EXISTS idx_users_last_login ON users(last_login_at DESC NULLS LAST);
CREATE INDEX IF NOT EXISTS idx_users_login_count ON users(login_count DESC);

-- Add comments
COMMENT ON COLUMN users.login_count IS 'Number of times user has logged in';
COMMENT ON COLUMN users.last_login_at IS 'Timestamp of user''s last successful login';
