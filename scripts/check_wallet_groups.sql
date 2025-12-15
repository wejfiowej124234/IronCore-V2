-- Quick check and add wallet_groups column manually
-- Execute this via any PostgreSQL client connected to CockroachDB

-- Step 1: Check if group_id column exists
SELECT column_name, data_type 
FROM information_schema.columns 
WHERE table_name = 'wallets' AND column_name = 'group_id';

-- Step 2: If not exists, add it (this will return nothing if column already exists due to IF NOT EXISTS)
ALTER TABLE wallets ADD COLUMN IF NOT EXISTS group_id UUID;

-- Step 3: Create wallet_groups table if not exists
CREATE TABLE IF NOT EXISTS wallet_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Step 4: Verify the changes
SELECT column_name, data_type 
FROM information_schema.columns 
WHERE table_name = 'wallets' AND column_name = 'group_id';

SELECT table_name FROM information_schema.tables WHERE table_name = 'wallet_groups';
