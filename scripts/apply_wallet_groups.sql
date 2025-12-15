-- Apply wallet_groups migration manually

-- 1. Create wallet_groups table
CREATE TABLE IF NOT EXISTS wallet_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    CONSTRAINT fk_wallet_groups_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    CONSTRAINT fk_wallet_groups_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- 2. Add group_id column to wallets table
ALTER TABLE wallets ADD COLUMN IF NOT EXISTS group_id UUID;

-- 3. Add foreign key constraint (only if not exists)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints 
        WHERE constraint_name = 'fk_wallets_group'
    ) THEN
        ALTER TABLE wallets 
        ADD CONSTRAINT fk_wallets_group 
        FOREIGN KEY (group_id) REFERENCES wallet_groups(id) ON DELETE CASCADE;
    END IF;
END $$;

-- 4. Create indexes
CREATE INDEX IF NOT EXISTS idx_wallet_groups_tenant_user ON wallet_groups(tenant_id, user_id);
CREATE INDEX IF NOT EXISTS idx_wallet_groups_name ON wallet_groups(name);
CREATE INDEX IF NOT EXISTS idx_wallets_group_id ON wallets(group_id);

-- 5. Create trigger function
CREATE OR REPLACE FUNCTION update_wallet_groups_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 6. Create trigger (drop first if exists)
DROP TRIGGER IF EXISTS trigger_wallet_groups_updated_at ON wallet_groups;
CREATE TRIGGER trigger_wallet_groups_updated_at
BEFORE UPDATE ON wallet_groups
FOR EACH ROW
EXECUTE FUNCTION update_wallet_groups_updated_at();

SELECT 'Wallet groups migration applied successfully!' AS status;
