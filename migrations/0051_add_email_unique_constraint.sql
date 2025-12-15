-- 添加email唯一约束防止并发注册竞态条件
-- P0 Critical Fix: 防止多个用户使用相同邮箱注册

-- 步骤1: 清理重复的email记录（保留最早的记录）
DELETE FROM users a
WHERE id NOT IN (
    SELECT MIN(id)
    FROM users
    GROUP BY email_cipher
);

-- 步骤2: 为email_cipher添加唯一约束（主要字段，生产环境加密使用）
ALTER TABLE users ADD CONSTRAINT users_email_cipher_unique UNIQUE (email_cipher);

-- 步骤3: 为email添加唯一约束（开发环境使用，作为备份）
-- 注意：如果email可能为NULL，需要使用部分索引
CREATE UNIQUE INDEX IF NOT EXISTS users_email_unique_idx ON users (email) WHERE email IS NOT NULL;

-- 步骤4: 添加索引加速email查询
CREATE INDEX IF NOT EXISTS idx_users_email_cipher ON users (email_cipher);
CREATE INDEX IF NOT EXISTS idx_users_email ON users (email) WHERE email IS NOT NULL;
