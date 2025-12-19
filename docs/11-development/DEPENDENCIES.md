# 📦 依赖库说明

> 项目使用的所有依赖库详解

## 📋 目录

- [核心框架](#核心框架)
- [数据库](#数据库)
- [区块链](#区块链)
- [加密安全](#加密安全)
- [工具库](#工具库)
- [开发工具](#开发工具)

---

## 核心框架

### axum (0.7)
**用途**: Web框架  
**为什么选择**: 
- 基于 tokio 的异步框架
- 类型安全的路由
- 零成本抽象
- 优秀的性能

**使用示例**:
```rust
use axum::{Router, routing::get};

let app = Router::new()
    .route("/api/health", get(health_check));
```

### tokio (1.37)
**用途**: 异步运行时  
**为什么选择**:
- Rust生态最流行的异步运行时
- 完整的异步I/O支持
- 高性能

**配置**:
```toml
tokio = { version = "1.37", features = [
    "rt-multi-thread",  # 多线程运行时
    "macros",           # #[tokio::main]
    "signal",           # 信号处理
    "time"              # 定时器
]}
```

### tower (0.5)
**用途**: 中间件框架  
**功能**: 
- 认证中间件
- 限流中间件
- 超时控制
- 服务组合

---

## 数据库

### sqlx (0.8)
**用途**: 异步SQL工具  
**为什么选择**:
- 编译时SQL验证
- 异步支持
- 防止SQL注入
- 支持PostgreSQL/CockroachDB

**配置**:
```toml
sqlx = { version = "0.8", features = [
    "runtime-tokio",    # Tokio异步运行时
    "postgres",         # PostgreSQL驱动
    "chrono",           # 时间类型
    "uuid",             # UUID类型
    "rust_decimal",     # 高精度数字
    "migrate"           # 数据库迁移
]}
```

**使用示例**:
```rust
let user = sqlx::query_as!(
    User,
    "SELECT * FROM users WHERE id = $1",
    user_id
)
.fetch_one(&pool)
.await?;
```

### redis (0.27)
**用途**: Redis客户端  
**功能**:
- 缓存
- 会话存储
- 限流计数器

**配置**:
```toml
redis = { version = "0.27", features = [
    "aio",          # 异步支持
    "tokio-comp"    # Tokio兼容
]}
```

---

## 区块链

### ethers (2.0)
**用途**: Ethereum客户端  
**功能**:
- 连接以太坊节点
- 发送交易
- 查询余额
- 智能合约交互

**配置**:
```toml
ethers = { version = "2.0", features = [
    "rustls",   # TLS支持
    "ws"        # WebSocket支持
]}
```

**使用示例**:
```rust
use ethers::providers::{Provider, Http};

let provider = Provider::<Http>::try_from(
    "https://mainnet.infura.io/v3/YOUR_KEY"
)?;
let block_number = provider.get_block_number().await?;
```

### bitcoin (0.31)
**用途**: Bitcoin客户端  
**功能**:
- 比特币地址生成
- 交易构建
- 脚本处理

**配置**:
```toml
bitcoin = { version = "0.31", features = ["serde"] }
```

### k256 (0.13) - Ethereum签名
**用途**: secp256k1椭圆曲线  
**功能**:
- 以太坊私钥/公钥
- ECDSA签名

**配置**:
```toml
k256 = { version = "0.13", features = [
    "ecdsa",    # 签名算法
    "sha256"    # 哈希
]}
```

### ed25519-dalek (2.1) - Solana/TON签名
**用途**: Ed25519签名算法  
**功能**:
- Solana私钥/公钥
- TON私钥/公钥

### schnorrkel (0.11) - Polkadot签名
**用途**: sr25519签名算法  
**功能**: Polkadot/Kusama签名

### bip39 (2.2)
**用途**: 助记词生成  
**功能**:
- 生成12/24词助记词
- 助记词转种子

**使用示例**:
```rust
use bip39::{Mnemonic, Language};

let mnemonic = Mnemonic::generate_in(Language::English, 12)?;
let seed = mnemonic.to_seed("");
```

### coins-bip32 (0.8)
**用途**: HD钱包派生  
**功能**:
- BIP32分层派生
- BIP44标准路径

---

## 加密安全

### bcrypt (0.15)
**用途**: 密码哈希  
**为什么选择**: 
- 慢速哈希算法
- 防止暴力破解
- 自动加盐

**使用示例**:
```rust
use bcrypt::{hash, verify};

let hashed = hash("password123", 10)?;
let valid = verify("password123", &hashed)?;
```

### aes-gcm (0.10)
**用途**: AES-256-GCM加密  
**功能**:
- 对称加密
- 认证加密（AEAD）

**使用示例**:
```rust
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};

let key = Key::<Aes256Gcm>::from_slice(key_bytes);
let cipher = Aes256Gcm::new(&key);
let nonce = Nonce::from_slice(nonce_bytes);

let ciphertext = cipher.encrypt(nonce, plaintext)?;
```

### jsonwebtoken (9.2)
**用途**: JWT认证  
**功能**:
- 生成JWT token
- 验证JWT token

**使用示例**:
```rust
use jsonwebtoken::{encode, decode, Header, Validation};

let token = encode(&Header::default(), &claims, &key)?;
let decoded = decode::<Claims>(&token, &key, &Validation::default())?;
```

### zeroize (1.6)
**用途**: 安全擦除内存  
**功能**:
- 防止私钥泄露
- 自动清零敏感数据

**使用示例**:
```rust
use zeroize::Zeroize;

let mut secret = String::from("sensitive data");
secret.zeroize();  // 清零内存
```

---

## 工具库

### serde (1.0)
**用途**: 序列化/反序列化  
**功能**:
- JSON序列化
- 结构体与JSON互转

**配置**:
```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**使用示例**:
```rust
#[derive(Serialize, Deserialize)]
struct User {
    name: String,
    email: String,
}

let json = serde_json::to_string(&user)?;
let user: User = serde_json::from_str(&json)?;
```

### uuid (1.7)
**用途**: UUID生成  
**配置**:
```toml
uuid = { version = "1.7", features = ["v4", "serde"] }
```

**使用示例**:
```rust
use uuid::Uuid;

let id = Uuid::new_v4();
```

### chrono (0.4)
**用途**: 日期时间处理  
**配置**:
```toml
chrono = { version = "0.4", features = ["serde", "clock"] }
```

### rust_decimal (1.35)
**用途**: 高精度小数  
**为什么选择**: 金融计算不能用float  
**使用场景**: 金额、Gas费、手续费

**使用示例**:
```rust
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

let amount = dec!(1.5);
let fee = dec!(0.001);
let total = amount + fee;  // 1.501
```

### anyhow (1.0)
**用途**: 错误处理  
**功能**: 
- 简化错误传播
- 错误上下文

**使用示例**:
```rust
use anyhow::{Result, Context};

fn read_config() -> Result<Config> {
    let content = fs::read_to_string("config.toml")
        .context("Failed to read config file")?;
    Ok(toml::from_str(&content)?)
}
```

### reqwest (0.12)
**用途**: HTTP客户端  
**功能**:
- 调用外部API
- RPC请求

**配置**:
```toml
reqwest = { version = "0.12", features = [
    "json",       # JSON支持
    "rustls-tls"  # TLS支持
]}
```

---

## 日志监控

### tracing (0.1)
**用途**: 结构化日志  
**为什么选择**:
- 比log更强大
- 支持分布式追踪
- 结构化输出

**使用示例**:
```rust
use tracing::{info, warn, error};

info!(user_id = %user.id, "User logged in");
warn!(retry_count = 3, "Retrying request");
error!(error = %e, "Database connection failed");
```

### tracing-subscriber (0.3)
**用途**: tracing后端  
**配置**:
```toml
tracing-subscriber = { version = "0.3", features = [
    "env-filter",  # 环境变量过滤
    "fmt",         # 格式化输出
    "json",        # JSON格式
    "chrono"       # 时间戳
]}
```

### prometheus (0.13)
**用途**: 监控指标  
**功能**:
- 请求计数
- 响应时间
- 错误率

---

## API文档

### utoipa (4)
**用途**: OpenAPI生成  
**功能**:
- 自动生成OpenAPI规范
- Swagger UI集成

**配置**:
```toml
utoipa = { version = "4", features = [
    "axum_extras",  # Axum集成
    "uuid",         # UUID类型
    "chrono"        # 时间类型
]}
```

**使用示例**:
```rust
#[utoipa::path(
    post,
    path = "/api/v1/wallets/batch",
    request_body = BatchCreateWalletsRequest,
    responses(
        (status = 200, description = "Success", body = ApiResponse<BatchCreateWalletsResponse>),
        (status = 400, description = "Bad Request")
    )
)]
async fn create_wallet() { }
```

---

## 开发工具

### 测试框架

#### tokio-test (0.4)
**用途**: 异步测试  
**使用示例**:
```rust
#[tokio::test]
async fn test_async_function() {
    let result = async_function().await;
    assert_eq!(result, expected);
}
```

#### mockall (0.12)
**用途**: Mock对象  
**使用示例**:
```rust
#[automock]
trait Database {
    async fn get_user(&self, id: Uuid) -> Result<User>;
}

#[tokio::test]
async fn test_with_mock() {
    let mut mock_db = MockDatabase::new();
    mock_db.expect_get_user()
        .returning(|_| Ok(User::default()));
}
```

#### criterion (0.5)
**用途**: 性能基准测试  
**使用示例**:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn fibonacci_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| {
        b.iter(|| fibonacci(black_box(20)))
    });
}

criterion_group!(benches, fibonacci_benchmark);
criterion_main!(benches);
```

---

## 配置管理

### toml (0.8)
**用途**: TOML解析  
**功能**: 读取config.toml

### config (0.14)
**用途**: 配置管理  
**功能**:
- 多源配置（文件+环境变量）
- 配置合并
- 类型安全

---

## 依赖选择原则

### 1. 性能优先
- 使用异步库（tokio生态）
- 避免阻塞操作
- 零成本抽象

### 2. 安全优先
- 使用经过审计的加密库
- 类型安全
- 内存安全

### 3. 维护性
- 选择活跃维护的库
- 避免过时的依赖
- 定期更新

### 4. 生态兼容
- 优先tokio生态
- 避免运行时冲突

---

## 依赖管理

### 检查过期依赖
```bash
cargo outdated
```

### 安全审计
```bash
cargo audit
```

### 更新依赖
```bash
# 更新到兼容版本
cargo update

# 更新到最新版本（需修改Cargo.toml）
cargo upgrade
```

---

## 性能对比

| 库 | 性能等级 | 内存占用 | 推荐指数 |
|---|---------|---------|---------|
| axum | ⚡⚡⚡⚡⚡ | 低 | ⭐⭐⭐⭐⭐ |
| sqlx | ⚡⚡⚡⚡ | 中 | ⭐⭐⭐⭐⭐ |
| redis | ⚡⚡⚡⚡⚡ | 低 | ⭐⭐⭐⭐⭐ |
| ethers | ⚡⚡⚡ | 高 | ⭐⭐⭐⭐ |
| bcrypt | ⚡⚡ | 低 | ⭐⭐⭐⭐⭐ |

---

## 相关文档

- [开发者指南](./DEVELOPER_GUIDE.md)
- [架构设计](../01-architecture/MULTI_CHAIN_WALLET_ARCHITECTURE.md)

---

**最后更新**: 2025-11-24  
**维护者**: Backend Team
