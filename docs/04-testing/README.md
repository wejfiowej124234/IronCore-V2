# 测试策略与实践 (Testing Strategy & Practices)

> 🧪 900+ 测试用例、单元测试、集成测试、性能测试

---

## 📂 本分类文档

| 文档 | 描述 | 状态 |
|------|------|------|
| [API_TESTING.md](./API_TESTING.md) | API 测试完整指南 | ✅ 核心 |
| [TESTING_FRAMEWORK.md](./TESTING_FRAMEWORK.md) | 测试框架设计 | ✅ 核心 |

---

## 🎯 快速导航

### 测试工程师
- 🧪 **[API 测试指南](./API_TESTING.md)** - 完整 API 测试流程
- 🏗️ **[测试框架](./TESTING_FRAMEWORK.md)** - 测试工具与方法

---

## 🧪 测试金字塔

```
         /\
        /  \  E2E Tests (5%)
       /────\  - Selenium/WebDriver
      /      \  - 完整业务流程
     /────────\
    / Integration \ Integration Tests (15%)
   /   Tests (15%) \ - API 集成测试
  /────────────────\ - Database 测试
 /                  \
/   Unit Tests (80%) \ Unit Tests (80%)
──────────────────────  - 函数级测试
     900+ Tests        - Mock 外部依赖
```

### 测试覆盖率目标

| 层级 | 覆盖率目标 | 当前状态 |
|------|-----------|----------|
| **总体代码覆盖率** | ≥ 80% | 85% ✅ |
| **Service 层** | ≥ 90% | 92% ✅ |
| **Repository 层** | ≥ 85% | 88% ✅ |
| **API Handler 层** | ≥ 75% | 78% ✅ |
| **关键路径** | 100% | 100% ✅ |

---

## 📚 测试文档详解

### 1️⃣ [API 测试指南](./API_TESTING.md) ⭐
**适合**: 后端工程师、测试工程师

**核心内容**:
- 🧪 **单元测试** - Service/Repository 层测试
- 🔗 **集成测试** - API 端到端测试
- 🎭 **Mock 策略** - 外部依赖 Mock
- 📊 **覆盖率报告** - llvm-cov 使用

**单元测试示例**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_create_wallet_success() {
        // Arrange
        let mut mock_repo = MockWalletRepository::new();
        mock_repo
            .expect_create()
            .with(eq(wallet_dto))
            .times(1)
            .returning(|_| Ok(wallet));

        let service = WalletService::new(mock_repo);

        // Act
        let result = service.create_wallet(request).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, wallet_id);
    }
}
```

**集成测试示例**:
```rust
#[tokio::test]
async fn test_wallet_api_integration() {
    // 1. 启动测试服务器
    let app = create_test_app().await;
    
    // 2. 注册用户
    let token = register_and_login(&app).await;
    
    // 3. 创建钱包
    let response = app
        .post("/api/wallets")
        .header("Authorization", format!("Bearer {}", token))
        .json(&create_wallet_request)
        .send()
        .await;
    
    // 4. 验证响应
    assert_eq!(response.status(), 201);
    let wallet: Wallet = response.json().await;
    assert_eq!(wallet.name, "Test Wallet");
}
```

**阅读时长**: 30 分钟

---

### 2️⃣ [测试框架](./TESTING_FRAMEWORK.md)
**适合**: 测试工程师、DevOps

**核心内容**:
- 🛠️ **测试工具** - tokio-test, mockall, wiremock
- 🎭 **Mock 框架** - 数据库、Redis、区块链 RPC
- 📊 **性能测试** - cargo bench, criterion
- 🔍 **测试数据管理** - fixtures, factory

**测试工具栈**:
| 工具 | 用途 | 文档 |
|------|------|------|
| `tokio-test` | 异步测试 | https://docs.rs/tokio-test |
| `mockall` | Mock 框架 | https://docs.rs/mockall |
| `wiremock` | HTTP Mock | https://docs.rs/wiremock |
| `sqlx-test` | 数据库测试 | https://docs.rs/sqlx |
| `criterion` | 性能基准测试 | https://docs.rs/criterion |

**阅读时长**: 20 分钟

---

## 🔍 测试最佳实践

### 1. 单元测试原则
- ✅ **Fast** - 快速执行（< 1s）
- ✅ **Independent** - 测试间独立
- ✅ **Repeatable** - 可重复执行
- ✅ **Self-Validating** - 自动验证
- ✅ **Timely** - 及时编写

### 2. 测试命名规范
```rust
#[tokio::test]
async fn test_<function>_<scenario>_<expected_result>() {
    // test_create_wallet_with_valid_data_returns_success
    // test_send_transaction_with_insufficient_balance_returns_error
}
```

### 3. AAA 模式
```rust
#[tokio::test]
async fn test_example() {
    // Arrange - 准备测试数据
    let user = create_test_user();
    let wallet = create_test_wallet();
    
    // Act - 执行被测试方法
    let result = service.transfer(from, to, amount).await;
    
    // Assert - 验证结果
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, "success");
}
```

### 4. Mock 外部依赖
```rust
// Mock 区块链 RPC
let mut mock_provider = MockProvider::new();
mock_provider
    .expect_get_balance()
    .returning(|_| Ok(U256::from(1_000_000_000)));

// Mock 数据库
let mut mock_repo = MockWalletRepository::new();
mock_repo
    .expect_find_by_id()
    .returning(|_| Ok(Some(wallet)));
```

---

## 📊 测试执行命令

### 运行所有测试
```bash
cd IronCore
cargo test --workspace
```

### 运行特定测试
```bash
# 运行单个测试
cargo test test_create_wallet_success

# 运行某个模块的测试
cargo test service::wallet

# 运行集成测试
cargo test --test integration_tests
```

### 生成覆盖率报告
```bash
# 安装 llvm-cov
cargo install cargo-llvm-cov

# 生成 HTML 覆盖率报告
cargo llvm-cov --html --open

# 生成 JSON 覆盖率报告
cargo llvm-cov --json --output-path coverage.json
```

### 运行性能基准测试
```bash
cd IronCore
cargo bench
```

---

## 📊 测试统计

### 测试数量分布

| 模块 | 单元测试 | 集成测试 | 总计 |
|------|---------|---------|------|
| Service | 350 | 50 | 400 |
| Repository | 200 | 30 | 230 |
| API Handler | 150 | 40 | 190 |
| Blockchain | 80 | 20 | 100 |
| Utils | 60 | 10 | 70 |
| **总计** | **840** | **150** | **990** |

### 测试执行时间

| 测试类型 | 平均时间 | 状态 |
|---------|---------|------|
| 单元测试 | 3.5s | ✅ |
| 集成测试 | 25s | ✅ |
| E2E 测试 | 120s | ✅ |
| 性能测试 | 60s | ✅ |

---

## 🔧 测试环境配置

### 测试数据库
```bash
# 使用内存 SQLite 测试（快速）
export DATABASE_URL="sqlite::memory:"

# 使用 Docker 测试数据库（真实环境）
docker run -d \
  --name ironcore-test-db \
  -p 5432:5432 \
  -e POSTGRES_DB=ironcore_test \
  -e POSTGRES_USER=test \
  -e POSTGRES_PASSWORD=test \
  postgres:15
```

### 测试配置文件
```toml
# config.test.toml
[server]
bind_addr = "127.0.0.1:0"  # 随机端口
allow_degraded_start = true

[database]
url = "sqlite::memory:"

[redis]
url = "redis://localhost:6379/1"  # 使用 DB 1

[jwt]
secret = "test-secret-for-testing-only"
token_expiry_secs = 300

[logging]
level = "debug"
```

---

## 🔗 相关文档

- **API 参考**: [03-api/API_REFERENCE.md](../03-api/API_REFERENCE.md)
- **错误处理**: [08-error-handling/ERROR_HANDLING.md](../08-error-handling/ERROR_HANDLING.md)
- **性能监控**: [07-monitoring/MONITORING.md](../07-monitoring/MONITORING.md)
- **CI/CD**: [11-development/CI_CD.md](../11-development/CI_CD.md)

---

**最后更新**: 2025-12-06  
**维护者**: QA & Testing Team  
**审查者**: Lead Test Engineer, Backend Lead
