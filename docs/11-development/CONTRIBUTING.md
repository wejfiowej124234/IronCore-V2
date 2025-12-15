# 🤝 贡献指南

> 欢迎为 IronForge Backend 项目做贡献！

## 📋 目录

- [行为准则](#行为准则)
- [我能做什么贡献](#我能做什么贡献)
- [开发流程](#开发流程)
- [代码规范](#代码规范)
- [提交规范](#提交规范)
- [Pull Request流程](#pull-request流程)
- [代码审查](#代码审查)
- [测试要求](#测试要求)

---

## 行为准则

### 我们的承诺

为了营造开放和友好的环境，我们承诺:

- ✅ 使用友好和包容的语言
- ✅ 尊重不同的观点和经验
- ✅ 优雅地接受建设性批评
- ✅ 关注对社区最有利的事情
- ✅ 对其他社区成员保持同理心

### 不可接受的行为

- ❌ 使用性化的语言或图像
- ❌ 人身攻击或侮辱性评论
- ❌ 骚扰行为
- ❌ 未经许可发布他人私人信息
- ❌ 其他不道德或不专业的行为

---

## 我能做什么贡献

### 1. 报告Bug 🐛

发现问题？请创建Issue:

**Issue模板**:
```markdown
**描述问题**
简要描述遇到的问题

**复现步骤**
1. 启动后端 `cargo run`
2. 调用API `POST /api/wallets`
3. 看到错误 `...`

**期望行为**
应该返回200和钱包对象

**实际行为**
返回500错误

**环境信息**
- OS: Windows 11
- Rust版本: 1.75.0
- 数据库: CockroachDB v23.1

**日志/截图**
```
[ERROR] Database connection failed
```

**相关代码**
`backend/src/api/handlers/wallet.rs:45`
```

### 2. 提出新功能 💡

有好主意？创建Feature Request:

**Feature模板**:
```markdown
**功能描述**
希望增加Solana钱包支持

**使用场景**
用户需要管理Solana资产

**实现建议**
1. 添加Ed25519签名支持
2. 实现Solana RPC客户端
3. 更新前端UI

**替代方案**
暂无

**优先级**
[ ] 高 [x] 中 [ ] 低
```

### 3. 贡献代码 💻

提交代码前请阅读本文档

### 4. 改进文档 📚

文档错误或不清楚？欢迎修正！

### 5. 代码审查 👀

帮助审查他人的PR

---

## 开发流程

### 1. Fork仓库

```bash
# 1. 在GitHub上Fork仓库
# https://github.com/your-org/ironforge-backend

# 2. Clone你的Fork
git clone https://github.com/YOUR_USERNAME/ironforge-backend.git
cd ironforge-backend

# 3. 添加上游仓库
git remote add upstream https://github.com/your-org/ironforge-backend.git
```

### 2. 创建分支

```bash
# 基于main创建功能分支
git checkout -b feature/add-solana-support

# 分支命名规范:
# - feature/xxx   - 新功能
# - fix/xxx       - Bug修复
# - docs/xxx      - 文档更新
# - refactor/xxx  - 重构
# - test/xxx      - 测试
# - chore/xxx     - 杂项（依赖更新等）
```

### 3. 开发

```bash
# 1. 安装依赖
cd backend
cargo build

# 2. 启动开发环境
docker compose -f ../ops/docker-compose.yml up -d
cargo run

# 3. 编写代码
# ...

# 4. 运行测试
cargo test

# 5. 代码格式化
cargo fmt

# 6. 代码检查
cargo clippy -- -D warnings
```

### 4. 提交代码

```bash
# 添加改动
git add .

# 提交（遵循Commit规范）
git commit -m "feat(wallet): add solana wallet support"

# 推送到你的Fork
git push origin feature/add-solana-support
```

### 5. 创建Pull Request

1. 访问你的Fork页面
2. 点击 "New Pull Request"
3. 填写PR描述
4. 等待代码审查

---

## 代码规范

### Rust代码风格

遵循标准Rust风格指南:

```rust
// ✅ 正确: 函数名使用snake_case
fn create_wallet() -> Result<Wallet> { ... }

// ❌ 错误: 不要使用camelCase
fn createWallet() -> Result<Wallet> { ... }

// ✅ 正确: 结构体使用PascalCase
struct WalletService { ... }

// ✅ 正确: 常量使用SCREAMING_SNAKE_CASE
const MAX_RETRY_COUNT: u32 = 3;

// ✅ 正确: 模块名使用snake_case
mod wallet_service;
```

### 文档注释

所有公开API必须有文档:

```rust
/// 创建新钱包
///
/// # 参数
///
/// * `user_id` - 用户ID
/// * `chain_id` - 链ID（1=Ethereum, 56=BSC）
/// * `name` - 钱包名称（可选）
///
/// # 返回
///
/// 返回创建的钱包对象
///
/// # 错误
///
/// * `DatabaseError` - 数据库操作失败
/// * `ValidationError` - 参数验证失败
///
/// # 示例
///
/// ```
/// let wallet = create_wallet(user_id, 1, Some("My Wallet")).await?;
/// ```
pub async fn create_wallet(
    user_id: Uuid,
    chain_id: i32,
    name: Option<String>,
) -> Result<Wallet> {
    // 实现...
}
```

### 错误处理

使用 `Result` 和 `anyhow`:

```rust
// ✅ 正确: 使用?传播错误
async fn get_user(id: Uuid) -> Result<User> {
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id)
        .fetch_one(&pool)
        .await?;
    Ok(user)
}

// ✅ 正确: 添加错误上下文
async fn read_config() -> Result<Config> {
    let content = fs::read_to_string("config.toml")
        .context("Failed to read config.toml")?;
    toml::from_str(&content)
        .context("Failed to parse config.toml")
}

// ❌ 错误: 不要使用unwrap
let user = get_user(id).await.unwrap();  // 会panic!

// ❌ 错误: 不要吞掉错误
let _ = get_user(id).await;  // 错误被忽略
```

### 异步代码

使用Tokio约定:

```rust
// ✅ 正确: 异步函数使用async/await
async fn fetch_balance(address: &str) -> Result<Decimal> {
    let provider = get_provider().await?;
    let balance = provider.get_balance(address).await?;
    Ok(balance)
}

// ✅ 正确: 并发请求使用join!
use tokio::join;

async fn fetch_all_balances() -> Result<(Decimal, Decimal)> {
    let (eth_balance, bsc_balance) = join!(
        fetch_eth_balance(),
        fetch_bsc_balance()
    );
    Ok((eth_balance?, bsc_balance?))
}

// ❌ 错误: 不要阻塞异步运行时
async fn bad_example() {
    std::thread::sleep(Duration::from_secs(1));  // 阻塞！
}

// ✅ 正确: 使用异步sleep
async fn good_example() {
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

---

## 提交规范

遵循 [Conventional Commits](https://www.conventionalcommits.org/):

### 格式

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type类型

- `feat`: 新功能
- `fix`: Bug修复
- `docs`: 文档更新
- `style`: 代码格式（不影响功能）
- `refactor`: 重构
- `perf`: 性能优化
- `test`: 测试
- `chore`: 构建/依赖更新

### 示例

```bash
# 新功能
git commit -m "feat(wallet): add solana wallet support"

# Bug修复
git commit -m "fix(api): handle null pointer in wallet creation"

# 文档
git commit -m "docs(readme): update installation instructions"

# 重构
git commit -m "refactor(service): extract wallet creation logic"

# 性能优化
git commit -m "perf(db): add index on wallets.user_id"

# Breaking Change（破坏性变更）
git commit -m "feat(api)!: change wallet API response format

BREAKING CHANGE: wallet endpoint now returns different JSON structure"
```

### Commit消息最佳实践

✅ **好的Commit**:
```
feat(wallet): add multi-signature wallet support

- Implement multi-sig wallet creation
- Add approval workflow
- Update database schema

Closes #123
```

❌ **不好的Commit**:
```
update code
fix bug
WIP
asdfsdf
```

---

## Pull Request流程

### 1. PR标题

使用Commit规范格式:
```
feat(wallet): add solana wallet support
```

### 2. PR描述模板

```markdown
## 变更类型
- [ ] Bug修复
- [x] 新功能
- [ ] 重构
- [ ] 文档更新

## 变更说明
实现Solana钱包支持，包括:
- Ed25519签名
- Solana RPC客户端
- 余额查询和转账

## 测试
- [x] 单元测试
- [x] 集成测试
- [ ] 手动测试

## 截图/日志
```
[INFO] Solana wallet created: 8xH7...
```

## 相关Issue
Closes #123

## 检查清单
- [x] 代码通过 `cargo clippy`
- [x] 代码通过 `cargo fmt --check`
- [x] 所有测试通过
- [x] 添加了文档注释
- [x] 更新了CHANGELOG.md
```

### 3. 等待审查

- 至少需要1个maintainer批准
- CI/CD检查必须通过
- 所有评论必须解决

### 4. 合并

Maintainer会使用以下方式合并:
- **Squash and Merge**: 小改动（默认）
- **Rebase and Merge**: 保留完整提交历史
- **Merge Commit**: 大功能分支

---

## 代码审查

### 审查者指南

审查时检查:

#### 1. 代码质量
- [ ] 遵循Rust风格指南
- [ ] 没有unwrap/panic（除非有注释说明）
- [ ] 正确的错误处理
- [ ] 没有不必要的clone/copy

#### 2. 功能正确性
- [ ] 实现符合需求
- [ ] 边界条件处理
- [ ] 并发安全

#### 3. 测试覆盖
- [ ] 核心逻辑有单元测试
- [ ] API有集成测试
- [ ] 测试用例充分

#### 4. 安全性
- [ ] 输入验证
- [ ] SQL注入防护
- [ ] 敏感数据加密

#### 5. 性能
- [ ] 没有N+1查询
- [ ] 合理使用缓存
- [ ] 异步I/O正确使用

### 评论示例

✅ **建设性评论**:
```markdown
这里可以使用 `map_err` 简化代码:

suggestion:
\```rust
let user = get_user(id)
    .await
    .map_err(|e| anyhow!("Failed to get user: {}", e))?;
\```
```

❌ **非建设性评论**:
```markdown
这代码太烂了
```

### 作者响应

- 感谢审查者的时间和建议
- 解释设计决策（如果有异议）
- 及时修复问题
- 标记已解决的评论为"Resolved"

---

## 测试要求

### 单元测试

每个服务/模块必须有单元测试:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_wallet() {
        let service = WalletService::new(mock_repo());
        let wallet = service.create_wallet(user_id, 1).await.unwrap();
        assert_eq!(wallet.chain_id, 1);
    }

    #[tokio::test]
    async fn test_create_wallet_invalid_chain() {
        let service = WalletService::new(mock_repo());
        let result = service.create_wallet(user_id, 999).await;
        assert!(result.is_err());
    }
}
```

### 集成测试

API端点必须有集成测试:

```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_create_wallet_api() {
    let app = setup_test_app().await;

    let response = app
        .post("/api/wallets")
        .json(&json!({
            "chain_id": 1,
            "name": "Test Wallet"
        }))
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let wallet: Wallet = response.json().await;
    assert_eq!(wallet.chain_id, 1);
}
```

### 测试覆盖率

- 核心业务逻辑: >80%
- API handlers: >70%
- 工具函数: >90%

运行覆盖率测试:
```bash
cargo tarpaulin --out Html --output-dir coverage
```

---

## 开发环境设置

### 必需工具

```bash
# 1. 安装Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. 安装数据库CLI
cargo install sqlx-cli --no-default-features --features postgres

# 3. 安装代码质量工具
rustup component add rustfmt clippy
```

### 推荐工具

```bash
# cargo-watch（自动重编译）
cargo install cargo-watch

# cargo-edit（依赖管理）
cargo install cargo-edit

# cargo-outdated（检查过期依赖）
cargo install cargo-outdated
```

### IDE配置

**VS Code**:
```json
// .vscode/settings.json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

---

## 常见问题

### 1. 如何同步上游更新？

```bash
# 拉取上游变更
git fetch upstream
git checkout main
git merge upstream/main

# 变基你的功能分支
git checkout feature/my-feature
git rebase main
```

### 2. Commit历史混乱怎么办？

```bash
# 交互式rebase整理commits
git rebase -i HEAD~5

# 在编辑器中:
# - pick: 保留commit
# - squash: 合并到上一个commit
# - reword: 修改commit消息
# - drop: 删除commit
```

### 3. 如何解决合并冲突？

```bash
# 1. 拉取最新main
git fetch upstream
git merge upstream/main

# 2. 解决冲突
# 编辑冲突文件，删除<<<< ==== >>>>标记

# 3. 标记为已解决
git add .

# 4. 完成合并
git merge --continue
```

### 4. PR被要求修改后如何更新？

```bash
# 1. 修改代码
# ...

# 2. 提交修改
git add .
git commit -m "fix: address review comments"

# 3. 推送到PR分支
git push origin feature/my-feature

# PR会自动更新
```

---

## 发布流程

### 版本号规范

遵循 [Semantic Versioning](https://semver.org/):

- `MAJOR.MINOR.PATCH` (例: 1.2.3)
- `MAJOR`: 破坏性变更
- `MINOR`: 新功能（向后兼容）
- `PATCH`: Bug修复

### 发布步骤

```bash
# 1. 更新版本号
# 编辑 Cargo.toml
version = "1.3.0"

# 2. 更新CHANGELOG.md
## [1.3.0] - 2025-01-24
### Added
- Solana wallet support
### Fixed
- Gas estimation bug

# 3. 提交
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release v1.3.0"

# 4. 创建标签
git tag -a v1.3.0 -m "Release v1.3.0"

# 5. 推送
git push origin main --tags
```

---

## 联系方式

- **GitHub Issues**: https://github.com/your-org/ironforge-backend/issues
- **Discord**: https://discord.gg/ironforge
- **Email**: dev@ironforge.io

---

## 致谢

感谢所有贡献者！ 🙏

查看贡献者列表: [CONTRIBUTORS.md](./CONTRIBUTORS.md)

---

**最后更新**: 2025-01-24  
**维护者**: Backend Team
