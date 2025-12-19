-- ============================================================================
-- 0046: 钱包组（Wallet Groups）
-- ============================================================================
-- 目的：支持多链钱包架构 - 1个助记词生成1个钱包组，包含4个链账户
-- 非托管设计：仅存储公钥和地址，助记词/私钥由用户端管理
-- ============================================================================

-- 1. 创建钱包组表
CREATE TABLE IF NOT EXISTS wallet_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    name TEXT NOT NULL,                           -- 钱包组名称（用户输入，如"CCTV1"）
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    CONSTRAINT fk_wallet_groups_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    CONSTRAINT fk_wallet_groups_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- 2. 为wallets表添加group_id外键（可选，允许NULL以兼容旧数据）
ALTER TABLE wallets 
ADD COLUMN IF NOT EXISTS group_id UUID;

ALTER TABLE wallets 
ADD CONSTRAINT fk_wallets_group 
FOREIGN KEY (group_id) REFERENCES wallet_groups(id) ON DELETE CASCADE;

-- 3. 索引优化
CREATE INDEX IF NOT EXISTS idx_wallet_groups_tenant_user ON wallet_groups(tenant_id, user_id);
CREATE INDEX IF NOT EXISTS idx_wallet_groups_name ON wallet_groups(name);
CREATE INDEX IF NOT EXISTS idx_wallets_group_id ON wallets(group_id);

-- 4. 注释
COMMENT ON TABLE wallet_groups IS '钱包组表：1个助记词对应1个钱包组，包含多个链账户。符合非托管架构：仅存储元数据，用户端管理私钥。';
COMMENT ON COLUMN wallet_groups.name IS '钱包组名称，如"CCTV1"（用户自定义）';
COMMENT ON COLUMN wallets.group_id IS '所属钱包组ID，NULL表示独立钱包（兼容旧数据）';

-- 5. 更新触发器（CockroachDB使用DEFAULT CURRENT_TIMESTAMP自动更新）
-- CockroachDB不支持plpgsql触发器，使用DEFAULT和应用层更新updated_at
