# 🧑‍💻 开发者指南

> 如何为 IronCore-V2（crate: ironcore）贡献代码

## 📋 目录

- [开发环境搭建](#开发环境搭建)
- [代码结构](#代码结构)
- [开发工作流](#开发工作流)
- [代码规范](#代码规范)
- [测试指南](#测试指南)
- [提交规范](#提交规范)
- [调试技巧](#调试技巧)

---

## 开发环境搭建

### 必需工具

```bash
# 1. Rust (1.75+)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustc --version  # 验证安装

# 2. 代码格式化工具
rustup component add rustfmt
rustup component add clippy

# 3. 数据库工具
cargo install sqlx-cli --no-default-features --features postgres

# 4. 开发工具（可选）
cargo install cargo-watch  # 自动重新编译
cargo install cargo-audit  # 安全审计
cargo install cargo-outdated  # 依赖检查
```

### IDE 推荐

**VS Code** (推荐):
```json
// .vscode/settings.json
{
  "rust-analyzer.cargo.allFeatures": true,
  "rust-analyzer.checkOnSave.command": "clippy",
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "rust-lang.rust-analyzer"
}
```

**推荐插件**:
- rust-analyzer
- Better TOML
- Error Lens
- GitLens

### 本地开发环境

```bash
# 1. 克隆项目
git clone <repo-url>
cd IronCore-V2

# 2. 复制配置文件
cp config.example.toml config.toml

# 3. 编辑配置（开发模式）
cat > config.toml << 'EOF'
[server]
bind_addr = "127.0.0.1:8088"
allow_degraded_start = true

[jwt]
secret = "dev-jwt-secret-min-32-chars-long-xxxxx"
token_expiry_secs = 3600

[logging]
level = "debug"
format = "text"

[monitoring]
enable_prometheus = false  # 开发环境可关闭
EOF

# 4. 启动开发服务器
cargo run
```

### 使用 cargo-watch 自动重载

```bash
# 代码变更自动重新编译
cargo watch -x run

# 清屏 + 运行测试
cargo watch -c -x test

# 清屏 + clippy检查
cargo watch -c -x clippy
```

---

## 代码结构

### 目录架构

```
IronCore-V2/
├── src/
│   ├── main.rs                    # 入口文件
│   ├── lib.rs                     # 库入口
│   ├── config.rs                  # 配置管理
│   ├── app_state.rs               # 应用状态
│   ├── error.rs                   # 错误类型
│   │
│   ├── api/                       # API 层（路由+处理器）
│   │   ├── mod.rs                 # 路由注册
│   │   ├── handlers.rs            # HTTP处理器
│   │   ├── admin_api.rs           # 管理员API
│   │   ├── multi_chain_api.rs     # 多链钱包API
│   │   ├── gas_api.rs             # Gas估算API
│   │   └── middleware/            # 中间件
│   │       ├── auth.rs            # 认证
│   │       ├── rate_limit.rs      # 限流
│   │       └── csrf.rs            # CSRF保护
│   │
│   ├── service/                   # 业务逻辑层
│   │   ├── users.rs               # 用户服务
│   │   ├── wallets.rs             # 钱包服务
│   │   ├── transactions.rs        # 交易服务
│   │   ├── fee_service.rs         # 费用服务
│   │   └── cross_chain_bridge_service.rs  # 跨链服务
│   │
│   ├── repository/                # 数据访问层
│   │   ├── users.rs               # 用户仓储
│   │   ├── wallets.rs             # 钱包仓储
│   │   ├── transactions.rs        # 交易仓储
│   │   └── policies.rs            # 策略仓储
│   │
│   ├── infrastructure/            # 基础设施层
│   │   ├── db.rs                  # 数据库连接
│   │   ├── cache.rs               # Redis缓存
│   │   ├── audit.rs               # Immudb审计
│   │   ├── monitoring.rs          # Prometheus监控
│   │   ├── logging.rs             # 日志系统
│   │   └── encryption.rs          # 加密工具
│   │
│   ├── domain/                    # 领域模型
│   │   ├── wallet.rs              # 钱包模型
│   │   ├── transaction.rs         # 交易模型
│   │   └── user.rs                # 用户模型
│   │
│   └── utils/                     # 工具函数
│       ├── crypto.rs              # 加密工具
│       └── validators.rs          # 验证器
│
├── tests/                         # 集成测试
│   ├── common/                    # 测试通用代码
│   └── integration_test.rs        # 集成测试
│
├── benches/                       # 性能测试
│   ├── performance_bench.rs       # 性能基准
│   └── fee_service_bench.rs       # 费用服务基准
│
├── migrations/                    # 数据库迁移
│   ├── 001_wallets.sql
│   └── ...
│
├── scripts/                       # 脚本工具
│   ├── setup/                     # 安装脚本
│   ├── test/                      # 测试脚本
│   └── utils/                     # 工具脚本
│
└── docs/                          # 文档
    └── ...
```

### 分层架构

```
┌────────────────────────────────────┐
│         API Layer (Axum)           │ ← HTTP请求入口
├────────────────────────────────────┤
│      Service Layer (业务逻辑)      │ ← 核心业务
├────────────────────────────────────┤
│    Repository Layer (数据访问)     │ ← 数据库操作
├────────────────────────────────────┤
│  Infrastructure (基础设施)         │ ← DB/Redis/Logging
└────────────────────────────────────┘
```

**依赖规则**: 上层可以依赖下层，下层不能依赖上层

---

## 开发工作流

### 1. 创建功能分支

```bash
# 从main分支创建功能分支
git checkout main
git pull origin main
git checkout -b feature/your-feature-name
```

### 2. 开发新功能

**示例: 添加新的API端点**

```rust
// ✅ 所有业务路由统一使用 /api/v1 前缀（health 例外：/api/health）
// 示例：复用现有的「非托管批量创建钱包」端点
use axum::{routing::post, Router};
use crate::api::wallet_batch_create_api::batch_create_wallets;

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/v1/wallets/batch", post(batch_create_wallets))
        .with_state(state)
}
```

### 3. 编写测试

```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_create_wallet() {
    let app = setup_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/wallets/batch")
                // 注意：该端点受 JWT 保护，测试中需带 Authorization: Bearer <token>
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"wallets":[{"chain":"ETH","address":"0x0000000000000000000000000000000000000000","public_key":"0x...","name":"Test Wallet"}]}"#
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}
```

### 4. 代码检查

```bash
# 格式化代码
cargo fmt

# Clippy检查
cargo clippy -- -D warnings

# 运行测试
cargo test

# 检查编译
cargo check --all-targets
```

### 5. 提交代码

```bash
git add .
git commit -m "feat: add wallet creation endpoint"
git push origin feature/your-feature-name
```

---

## 代码规范

### Rust 编码风格

**1. 命名规范**

```rust
// ✅ 正确
struct UserAccount { }        // PascalCase for types
fn create_user() { }          // snake_case for functions
const MAX_RETRIES: u32 = 3;   // SCREAMING_SNAKE_CASE for constants

// ❌ 错误
struct user_account { }       // 应该用 PascalCase
fn CreateUser() { }           // 应该用 snake_case
const maxRetries: u32 = 3;    // 应该用 SCREAMING_SNAKE_CASE
```

**2. 错误处理**

```rust
// ✅ 正确: 使用 Result
pub async fn get_user(id: Uuid) -> Result<User, AppError> {
    let user = repository::get_user(id).await?;
    Ok(user)
}

// ❌ 错误: 使用 unwrap/expect (除非在测试中)
pub async fn get_user(id: Uuid) -> User {
    repository::get_user(id).await.unwrap()
}
```

**3. 异步函数**

```rust
// ✅ 正确: 使用 async/await
pub async fn fetch_data() -> Result<Data> {
    let response = reqwest::get("https://api.example.com")
        .await?
        .json::<Data>()
        .await?;
    Ok(response)
}

// ❌ 错误: 阻塞调用
pub fn fetch_data() -> Result<Data> {
    let response = reqwest::blocking::get("https://api.example.com")?
        .json::<Data>()?;
    Ok(response)
}
```

**4. 文档注释**

```rust
/// 创建新用户
///
/// # Arguments
/// * `name` - 用户名称
/// * `email` - 用户邮箱
///
/// # Returns
/// 创建的用户对象
///
/// # Errors
/// - 如果邮箱已存在，返回 `AppError::DuplicateEmail`
/// - 如果数据库连接失败，返回 `AppError::DatabaseError`
///
/// # Example
/// ```
/// let user = create_user("Alice", "alice@example.com").await?;
/// ```
pub async fn create_user(name: &str, email: &str) -> Result<User, AppError> {
    // ...
}
```

### 项目特定规范

**1. 依赖注入**

```rust
// ✅ 正确: 通过 AppState 注入依赖
pub async fn handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Response>, AppError> {
    let result = service::do_something(&state.pool).await?;
    Ok(Json(result))
}

// ❌ 错误: 直接创建连接
pub async fn handler() -> Result<Json<Response>, AppError> {
    let pool = PgPool::connect("...").await?;  // 不要这样做
    // ...
}
```

**2. 日志记录**

```rust
use tracing::{info, warn, error, debug};

// ✅ 正确: 使用结构化日志
info!(user_id = %user.id, action = "create_wallet", "Wallet created successfully");

// ❌ 错误: 使用字符串插值
println!("Wallet created for user {}", user.id);
```

**3. 配置管理**

```rust
// ✅ 正确: 从配置读取
let timeout = config.database.timeout_secs;

// ❌ 错误: 硬编码
let timeout = 30;
```

---

## 测试指南

### 单元测试

```rust
// src/service/fee_service.rs
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_calculate_flat_fee() {
        let fee = calculate_fee(
            FeeType::Flat,
            dec!(1.0),
            Some(dec!(0.001)),
            None,
        );
        assert_eq!(fee, dec!(0.001));
    }

    #[test]
    fn test_calculate_percent_fee() {
        let fee = calculate_fee(
            FeeType::Percent,
            dec!(1.0),
            None,
            Some(10), // 0.1%
        );
        assert_eq!(fee, dec!(0.001));
    }
}
```

### 集成测试

```rust
// tests/integration_test.rs
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn test_health_check() {
    let app = setup_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}
```

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test fee_service

# 运行单个测试
cargo test test_calculate_flat_fee

# 显示输出
cargo test -- --nocapture

# 并行度控制
cargo test -- --test-threads=1

# 集成测试
cargo test --test integration_test

# 性能测试
cargo bench
```

### 测试覆盖率

```bash
# 安装 tarpaulin
cargo install cargo-tarpaulin

# 生成覆盖率报告
cargo tarpaulin --out Html --output-dir coverage
```

---

## 提交规范

### Commit Message 格式

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Type**:
- `feat`: 新功能
- `fix`: Bug修复
- `docs`: 文档更新
- `style`: 代码格式（不影响代码运行）
- `refactor`: 重构
- `perf`: 性能优化
- `test`: 测试相关
- `chore`: 构建/工具链

**示例**:
```bash
feat(api): add wallet creation endpoint

Implement POST /api/v1/wallets/batch endpoint with the following features:
- Support multiple chains
- Validate wallet address format
- Store wallet metadata in database

Closes #123
```

### Pull Request 规范

**标题格式**:
```
[Type] Brief description
```

**PR 描述模板**:
```markdown
## 变更类型
- [ ] 新功能
- [ ] Bug修复
- [ ] 重构
- [ ] 文档更新

## 变更描述
简要描述这次PR的目的和实现

## 测试
- [ ] 单元测试已通过
- [ ] 集成测试已通过
- [ ] 手动测试已完成

## 相关Issue
Closes #123

## 检查清单
- [ ] 代码已格式化 (cargo fmt)
- [ ] Clippy检查通过 (cargo clippy)
- [ ] 测试覆盖率充足
- [ ] 文档已更新
```

---

## 调试技巧

### 1. 使用 dbg! 宏

```rust
fn calculate_total(items: &[Item]) -> f64 {
    let total = items.iter()
        .map(|item| dbg!(item.price))  // 打印每个价格
        .sum();
    dbg!(total)  // 打印总价
}
```

### 2. 日志调试

```rust
use tracing::{debug, info, warn, error};

async fn process_transaction(tx: Transaction) -> Result<()> {
    debug!(?tx, "Processing transaction");
    
    let result = db::save_transaction(&tx).await;
    
    match result {
        Ok(_) => info!(tx_id = %tx.id, "Transaction saved"),
        Err(e) => error!(error = %e, "Failed to save transaction"),
    }
    
    Ok(())
}
```

### 3. 使用 VS Code 调试器

**.vscode/launch.json**:
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug backend",
      "cargo": {
                "args": ["build", "--bin=ironcore"]
      },
      "args": [],
            "cwd": "${workspaceFolder}"
    }
  ]
}
```

### 4. 环境变量调试

```bash
# 启用详细日志
RUST_LOG=debug cargo run

# 启用特定模块日志
RUST_LOG=ironcore::service=debug cargo run

# 显示SQL查询
RUST_LOG=sqlx=debug cargo run

# 显示backtrace
RUST_BACKTRACE=1 cargo run

# 完整backtrace
RUST_BACKTRACE=full cargo run
```

### 5. 性能分析

```bash
# 安装 flamegraph
cargo install flamegraph

# 生成火焰图
cargo flamegraph --bin ironcore

# 使用 criterion 基准测试
cargo bench
```

---

## 常用命令速查

```bash
# 开发
cargo run                          # 运行程序
cargo watch -x run                 # 自动重载
cargo check                        # 快速检查

# 测试
cargo test                         # 运行测试
cargo test -- --nocapture         # 显示输出
cargo bench                        # 性能测试

# 代码质量
cargo fmt                          # 格式化
cargo clippy -- -D warnings       # Lint检查
cargo audit                        # 安全审计
cargo outdated                     # 检查过期依赖

# 构建
cargo build                        # Debug构建
cargo build --release             # Release构建
cargo clean                        # 清理

# 文档
cargo doc --open                   # 生成并打开文档
```

---

## 相关文档

- [架构设计](../01-architecture/MULTI_CHAIN_WALLET_ARCHITECTURE.md)
- [API文档](../01-architecture/API_ROUTES_MAP.md)
- [测试策略](../04-testing/MULTI_CHAIN_WALLET_TEST_REPORT.md)
- [错误处理](../08-error-handling/ERROR_HANDLING.md)

---

**最后更新**: 2025-11-24  
**维护者**: Backend Team
