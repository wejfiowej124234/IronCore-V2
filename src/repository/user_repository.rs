// 用户数据访问 Repository
// 提供 trait 接口支持 mock 测试

use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

// ============ 领域模型 ============

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email_cipher: String,
    pub email: Option<String>, // ✅ 新增（开发环境）
    pub phone_cipher: Option<String>,
    pub phone: Option<String>, // ✅ 新增（开发环境）
    pub role: String,
    pub status: String, // ✅ 新增
    pub password_hash: String,
    pub kyc_status: String, // ✅ 新增
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateUserParams {
    pub tenant_id: Uuid,
    pub email_cipher: String,
    pub phone_cipher: Option<String>,
    pub role: String,
    pub password_hash: String,
}

// ============ Repository Trait ============

#[async_trait]
pub trait UserRepository: Send + Sync {
    /// 根据 ID 查询用户
    async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>>;

    /// 根据邮箱查询用户
    async fn find_by_email(&self, email: &str) -> Result<Option<User>>;

    /// 创建新用户
    async fn create(&self, params: CreateUserParams) -> Result<User>;

    /// 更新用户角色
    async fn update_role(&self, user_id: Uuid, new_role: &str) -> Result<()>;

    /// 更新密码哈希
    async fn update_password_hash(&self, user_id: Uuid, new_hash: &str) -> Result<()>;

    /// 删除用户（软删除）
    async fn delete(&self, user_id: Uuid) -> Result<()>;

    /// 列出租户下的所有用户
    async fn list_by_tenant(&self, tenant_id: Uuid, limit: i64, offset: i64) -> Result<Vec<User>>;
}

// ============ PostgreSQL 实现 ============

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>> {
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                String,
                String,
                String,
                String,
                chrono::DateTime<chrono::Utc>,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
            "SELECT id, tenant_id, email_cipher, email, phone_cipher, phone, 
                    role, status, password_hash, kyc_status, created_at, updated_at
             FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(
                id,
                tenant_id,
                email_cipher,
                email,
                phone_cipher,
                phone,
                role,
                status,
                password_hash,
                kyc_status,
                created_at,
                updated_at,
            )| {
                User {
                    id,
                    tenant_id,
                    email_cipher,
                    email,
                    phone_cipher,
                    phone,
                    role,
                    status,
                    password_hash,
                    kyc_status,
                    created_at,
                    updated_at,
                }
            },
        ))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                String,
                String,
                String,
                String,
                chrono::DateTime<chrono::Utc>,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
            "SELECT id, tenant_id, email_cipher, email, phone_cipher, phone, 
                    role, status, password_hash, kyc_status, created_at, updated_at
             FROM users WHERE email_cipher = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(
                id,
                tenant_id,
                email_cipher,
                email,
                phone_cipher,
                phone,
                role,
                status,
                password_hash,
                kyc_status,
                created_at,
                updated_at,
            )| {
                User {
                    id,
                    tenant_id,
                    email_cipher,
                    email,
                    phone_cipher,
                    phone,
                    role,
                    status,
                    password_hash,
                    kyc_status,
                    created_at,
                    updated_at,
                }
            },
        ))
    }

    async fn create(&self, params: CreateUserParams) -> Result<User> {
        let user_id = Uuid::new_v4();

        // 使用RETURNING子句，CockroachDB完全支持，避免额外的查询
        let row = sqlx::query_as::<_, (
            Uuid, Uuid, String, Option<String>, Option<String>, Option<String>,
            String, String, String, String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>
        )>(
            "INSERT INTO users (id, tenant_id, email_cipher, phone_cipher, role, password_hash)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, tenant_id, email_cipher, email, phone_cipher, phone, role, status, password_hash, kyc_status, created_at, updated_at"
        )
        .bind(user_id)
        .bind(params.tenant_id)
        .bind(&params.email_cipher)
        .bind(&params.phone_cipher)
        .bind(&params.role)
        .bind(&params.password_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(User {
            id: row.0,
            tenant_id: row.1,
            email_cipher: row.2,
            email: row.3,
            phone_cipher: row.4,
            phone: row.5,
            role: row.6,
            status: row.7,
            password_hash: row.8,
            kyc_status: row.9,
            created_at: row.10,
            updated_at: row.11,
        })
    }

    async fn update_role(&self, user_id: Uuid, new_role: &str) -> Result<()> {
        sqlx::query("UPDATE users SET role = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
            .bind(new_role)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_password_hash(&self, user_id: Uuid, new_hash: &str) -> Result<()> {
        sqlx::query(
            "UPDATE users SET password_hash = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(new_hash)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete(&self, user_id: Uuid) -> Result<()> {
        // 软删除：更新 deleted_at 字段（需要先添加此字段到表）
        // 这里简化为标记角色为 "deleted"
        sqlx::query(
            "UPDATE users SET role = 'deleted', updated_at = CURRENT_TIMESTAMP WHERE id = $1",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_by_tenant(&self, tenant_id: Uuid, limit: i64, offset: i64) -> Result<Vec<User>> {
        let rows = sqlx::query_as::<_, (
            Uuid, Uuid, String, Option<String>, Option<String>, Option<String>,
            String, String, String, String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>
        )>(
            "SELECT id, tenant_id, email_cipher, email, phone_cipher, phone, role, status, password_hash, kyc_status, created_at, updated_at
             FROM users 
             WHERE tenant_id = $1 AND role != 'deleted'
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3"
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    tenant_id,
                    email_cipher,
                    email,
                    phone_cipher,
                    phone,
                    role,
                    status,
                    password_hash,
                    kyc_status,
                    created_at,
                    updated_at,
                )| {
                    User {
                        id,
                        tenant_id,
                        email_cipher,
                        email,
                        phone_cipher,
                        phone,
                        role,
                        status,
                        password_hash,
                        kyc_status,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect())
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    // mockall is not configured in Cargo.toml, so we comment out these tests
    // use mockall::predicate::*;

    #[tokio::test]
    #[ignore]
    async fn test_mock_user_repository() {
        // TODO: Enable mockall in Cargo.toml to use this test
        // let mut mock_repo = MockUserRepository::new();
        // let user_id = Uuid::new_v4();
        //
        // 设置 mock 预期
        // mock_repo
        // .expect_find_by_id()
        // .with(eq(user_id))
        // .times(1)
        // .returning(move |_| {
        // Ok(Some(User {
        // id: user_id,
        // tenant_id: Uuid::new_v4(),
        // email_cipher: "test@example.com".to_string(),
        // phone_cipher: None,
        // role: "user".to_string(),
        // password_hash: "hash123".to_string(),
        // created_at: chrono::Utc::now(),
        // updated_at: chrono::Utc::now(),
        // }))
        // });
        //
        // 测试
        // let result = mock_repo.find_by_id(user_id).await;
        // assert!(result.is_ok());
        // assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    #[ignore]
    async fn test_mock_create_user() {
        // TODO: Enable mockall in Cargo.toml to use this test
        // let mut mock_repo = MockUserRepository::new();
        // let tenant_id = Uuid::new_v4();
        //
        // mock_repo
        // .expect_create()
        // .times(1)
        // .returning(move |params| {
        // Ok(User {
        // id: Uuid::new_v4(),
        // tenant_id: params.tenant_id,
        // email_cipher: params.email_cipher,
        // phone_cipher: params.phone_cipher,
        // role: params.role,
        // password_hash: params.password_hash,
        // created_at: chrono::Utc::now(),
        // updated_at: chrono::Utc::now(),
        // })
        // });
        //
        // let params = CreateUserParams {
        // tenant_id,
        // email_cipher: "new@example.com".to_string(),
        // phone_cipher: None,
        // role: "user".to_string(),
        // password_hash: "hash456".to_string(),
        // };
        //
        // let result = mock_repo.create(params).await;
        // assert!(result.is_ok());
    }
}
