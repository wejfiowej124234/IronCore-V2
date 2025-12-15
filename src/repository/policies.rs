use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::infrastructure::db::PgPool;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Policy {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub rules: serde_json::Value,
    pub version: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub struct CreatePolicyInput {
    pub tenant_id: Uuid,
    pub name: String,
    pub rules: serde_json::Value,
    pub version: Option<i32>,
}

pub async fn create(pool: &PgPool, input: CreatePolicyInput) -> Result<Policy, sqlx::Error> {
    let version = input.version.unwrap_or(1);
    let rec = sqlx::query_as::<_, Policy>(
        r#"
        INSERT INTO policies (tenant_id, name, rules, version)
        VALUES ($1, $2, $3, $4)
        RETURNING id, tenant_id, name, rules, version, created_at
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.name)
    .bind(input.rules)
    .bind(version)
    .fetch_one(pool)
    .await?;
    Ok(rec)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Policy>, sqlx::Error> {
    let rec = sqlx::query_as::<_, Policy>(
        r#"
        SELECT id, tenant_id, name, rules, version, created_at
        FROM policies
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn list_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Policy>, sqlx::Error> {
    let recs = sqlx::query_as::<_, Policy>(
        r#"
        SELECT id, tenant_id, name, rules, version, created_at
        FROM policies
        WHERE tenant_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(tenant_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(recs)
}

pub async fn update(
    pool: &PgPool,
    id: Uuid,
    tenant_id: Uuid,
    name: Option<String>,
    rules: Option<serde_json::Value>,
    version: Option<i32>,
) -> Result<Option<Policy>, sqlx::Error> {
    // 如果所有字段都是None，直接返回当前记录
    if name.is_none() && rules.is_none() && version.is_none() {
        return get_by_id(pool, id).await;
    }

    // 构建更新查询
    let rec = if name.is_some() && rules.is_some() && version.is_some() {
        sqlx::query_as::<_, Policy>(
            r#"
            UPDATE policies SET name = $3, rules = $4, version = $5
            WHERE id = $1 AND tenant_id = $2
            RETURNING id, tenant_id, name, rules, version, created_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(name)
        .bind(rules)
        .bind(version)
        .fetch_optional(pool)
        .await?
    } else if rules.is_some() && version.is_some() {
        // 更新 rules 和 version
        sqlx::query_as::<_, Policy>(
            r#"
            UPDATE policies SET rules = $3, version = $4
            WHERE id = $1 AND tenant_id = $2
            RETURNING id, tenant_id, name, rules, version, created_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(rules.expect("rules checked above"))
        .bind(version.expect("version checked above"))
        .fetch_optional(pool)
        .await?
    } else if name.is_some() && version.is_some() {
        // 更新 name 和 version
        sqlx::query_as::<_, Policy>(
            r#"
            UPDATE policies SET name = $3, version = $4
            WHERE id = $1 AND tenant_id = $2
            RETURNING id, tenant_id, name, rules, version, created_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(name.expect("name checked above"))
        .bind(version.expect("version checked above"))
        .fetch_optional(pool)
        .await?
    } else if rules.is_some() {
        // 仅更新 rules
        sqlx::query_as::<_, Policy>(
            r#"
            UPDATE policies SET rules = $3
            WHERE id = $1 AND tenant_id = $2
            RETURNING id, tenant_id, name, rules, version, created_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(rules.expect("rules checked above"))
        .fetch_optional(pool)
        .await?
    } else if version.is_some() {
        // 仅更新 version
        sqlx::query_as::<_, Policy>(
            r#"
            UPDATE policies SET version = $3
            WHERE id = $1 AND tenant_id = $2
            RETURNING id, tenant_id, name, rules, version, created_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(version.expect("version checked above"))
        .fetch_optional(pool)
        .await?
    } else {
        // 没有提供任何更新参数，返回 None
        None
    };
    Ok(rec)
}

pub async fn delete(pool: &PgPool, id: Uuid, tenant_id: Uuid) -> Result<bool, sqlx::Error> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM policies
        WHERE id = $1 AND tenant_id = $2
        "#,
    )
    .bind(id)
    .bind(tenant_id)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows_affected > 0)
}

pub async fn count_by_tenant(pool: &PgPool, tenant_id: Uuid) -> Result<i64, sqlx::Error> {
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM policies WHERE tenant_id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await?;
    Ok(count.0)
}
