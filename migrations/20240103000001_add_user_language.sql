-- Add language column to users table
ALTER TABLE users 
ADD COLUMN language TEXT NOT NULL DEFAULT 'en';

-- Add check constraint for valid languages
ALTER TABLE users
ADD CONSTRAINT users_language_check 
CHECK (language IN ('en', 'pl', 'uk', 'ru'));

-- Create index for language queries (optional, но полезно для аналитики)
CREATE INDEX idx_users_language ON users(language);
