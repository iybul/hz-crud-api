-- Add unique constraint on email
ALTER TABLE organizations ADD CONSTRAINT unique_org_email UNIQUE (email);

-- Add password hash and salt fields to organizations table
ALTER TABLE organizations ADD COLUMN password_hash VARCHAR NOT NULL DEFAULT '';
ALTER TABLE organizations ADD COLUMN password_salt VARCHAR NOT NULL DEFAULT '';

-- Add access tokens table for authentication
CREATE TABLE access_tokens (
    id SERIAL PRIMARY KEY,
    token VARCHAR NOT NULL UNIQUE,
    org_id INTEGER NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,
    is_revoked BOOLEAN NOT NULL DEFAULT FALSE
);

-- Create index on token for faster lookups
CREATE INDEX idx_access_tokens_token ON access_tokens(token);

-- Add a demo organization with a hashed password for testing
-- Password is 'password' - DO NOT USE IN PRODUCTION
INSERT INTO organizations (name, email, password_hash, password_salt)
VALUES (
    'Demo Organization', 
    'demo@example.com',
    -- This is a pre-generated Argon2 hash of 'password'
    '$argon2id$v=19$m=16,t=2,p=1$a2JHUktSeU9VdEpYWmRONA$p5/f/GZ9lFPkPxOgVSn4Uw',
    'kbGRKRyOUtJXZdN4'
) ON CONFLICT (email) DO NOTHING;