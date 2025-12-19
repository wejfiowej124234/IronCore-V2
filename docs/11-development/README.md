# 开发指南 (Development Guide)

> 💻 开发环境、代码规范、Git 工作流、CI/CD、贡献指南

---

## 📂 本分类文档

| 文档 | 描述 | 状态 |
|------|------|------|
| [DEVELOPMENT_GUIDE.md](./DEVELOPMENT_GUIDE.md) | 完整开发指南 | ✅ 核心 |
| [CODE_STANDARDS.md](./CODE_STANDARDS.md) | 代码规范 | ✅ 核心 |
| [CI_CD.md](./CI_CD.md) | CI/CD 配置 | ✅ 核心 |
| [CONTRIBUTING.md](../../CONTRIBUTING.md) | 贡献指南 | ✅ 核心 |

---

## 🎯 快速导航

### 新开发者
- 🚀 **[开发指南](./DEVELOPMENT_GUIDE.md)** - 从零开始开发
- 📝 **[代码规范](./CODE_STANDARDS.md)** - 编码标准

### DevOps 工程师
- 🔄 **[CI/CD 配置](./CI_CD.md)** - 自动化流程

### 贡献者
- 🤝 **[贡献指南](../../CONTRIBUTING.md)** - 如何贡献代码

---

## 💻 开发环境

### 必备工具

```
开发工具栈 (Development Stack)
  ├─ Rust 1.75+ (stable)
  ├─ Cargo (Rust 包管理器)
  ├─ rustfmt (代码格式化)
  ├─ clippy (代码检查)
  ├─ cargo-watch (文件监听)
  └─ cargo-llvm-cov (代码覆盖率)

数据库工具
  ├─ CockroachDB 23.1+ 或 PostgreSQL 15+
  ├─ sqlx-cli (数据库迁移)
  └─ DBeaver/DataGrip (数据库客户端)

辅助工具
  ├─ Docker & Docker Compose
  ├─ Redis
  ├─ Immudb
  └─ Postman/Insomnia (API 测试)

IDE 推荐
  ├─ VS Code + rust-analyzer
  ├─ IntelliJ IDEA + Rust Plugin
  └─ Vim/Neovim + rust.vim
```

### 环境搭建

```bash
# 1. 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. 安装开发工具
cargo install sqlx-cli --no-default-features --features postgres
cargo install cargo-watch
cargo install cargo-llvm-cov

# 3. 克隆仓库
git clone https://github.com/your-org/ironcore.git
cd ironcore/IronCore-V2

# 4. 配置环境变量
cp .env.example .env
vim .env

# 5. 启动基础设施
docker compose -f ../ops/docker-compose.yml up -d

# 6. 运行数据库迁移
sqlx migrate run

# 7. 启动后端
cargo run

# 8. 验证
curl http://localhost:8088/api/health
```

---

## 📚 开发文档详解

### 1️⃣ [开发指南](./DEVELOPMENT_GUIDE.md) ⭐
**适合**: 所有开发人员

**核心内容**:
- 🚀 **快速开始** - 5 分钟启动项目
- 🔧 **开发工作流** - 日常开发流程
- 🧪 **测试驱动开发** - TDD 实践
- 📊 **性能调优** - Profiling 与优化

**开发工作流**:
```
1. 创建功能分支
   git checkout -b feature/new-api

2. 编写代码 + 单元测试
   vim src/api/handlers/new_handler.rs
   vim src/api/handlers/new_handler_test.rs

3. 运行测试
   cargo test --workspace

4. 格式化代码
   cargo fmt

5. 静态检查
   cargo clippy -- -D warnings

6. 提交代码
   git add .
   git commit -m "feat: Add new API endpoint"

7. 推送分支
   git push origin feature/new-api

8. 创建 Pull Request
   在 GitHub 上创建 PR
```

**阅读时长**: 40 分钟

---

### 2️⃣ [代码规范](./CODE_STANDARDS.md) ⭐
**适合**: 所有开发人员

**核心内容**:
- 📝 **命名规范** - 变量、函数、模块命名
- 🎨 **代码风格** - rustfmt 配置
- 📦 **模块组织** - 项目结构规范
- 📄 **文档规范** - 代码注释标准

**命名规范**:
```rust
// ✅ 好的命名
pub struct WalletService {
    repository: Arc<dyn WalletRepository>,
}

impl WalletService {
    pub async fn create_wallet(&self, request: CreateWalletRequest) -> Result<Wallet> {
        // ...
    }
}

// ❌ 不好的命名
pub struct WS {  // 缩写不清晰
    repo: Arc<dyn WR>,
}

impl WS {
    pub async fn cw(&self, req: CWR) -> Result<W> {  // 缩写过度
        // ...
    }
}
```

**代码风格**:
```toml
# rustfmt.toml
max_width = 100
hard_tabs = false
tab_spaces = 4
edition = "2021"
use_field_init_shorthand = true
use_try_shorthand = true
```

**阅读时长**: 30 分钟

---

### 3️⃣ [CI/CD 配置](./CI_CD.md) ⭐
**适合**: DevOps, 后端工程师

**核心内容**:
- 🔄 **GitHub Actions** - 自动化流程
- 🧪 **自动化测试** - 每次提交运行测试
- 📦 **自动化构建** - Docker 镜像构建
- 🚀 **自动化部署** - 部署到 Kubernetes

**GitHub Actions 配置**:
```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
          components: rustfmt, clippy
      
      - name: Cache cargo
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Run tests
        run: cargo test --workspace
      
      - name: Run clippy
        run: cargo clippy -- -D warnings
      
      - name: Check formatting
        run: cargo fmt -- --check
      
      - name: Build
        run: cargo build --release

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Generate coverage
        run: cargo llvm-cov --workspace --lcov --output-path lcov.info
      - name: Upload to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: lcov.info
```

**阅读时长**: 35 分钟

---

### 4️⃣ [贡献指南](../../CONTRIBUTING.md)
**适合**: 开源贡献者

**核心内容**:
- 🤝 **如何贡献** - 贡献流程
- 📋 **Issue 模板** - Bug 报告、功能请求
- 🔀 **PR 模板** - Pull Request 规范
- 👥 **Code Review** - 代码审查流程

**贡献流程**:
```
1. Fork 仓库
   在 GitHub 上 Fork 项目

2. 克隆到本地
   git clone https://github.com/your-username/ironcore.git

3. 创建分支
   git checkout -b feature/your-feature

4. 提交代码
   git add .
   git commit -m "feat: Your feature description"

5. 推送分支
   git push origin feature/your-feature

6. 创建 Pull Request
   在 GitHub 上创建 PR

7. Code Review
   等待 Maintainer 审查

8. 合并代码
   审查通过后合并到 main
```

**阅读时长**: 20 分钟

---

## 🔍 代码质量检查

### 必须通过的检查

```bash
# 1. 格式化检查
cargo fmt -- --check

# 2. 静态检查
cargo clippy -- -D warnings

# 3. 单元测试
cargo test --workspace

# 4. 集成测试
cargo test --test integration_tests

# 5. 覆盖率检查 (> 80%)
cargo llvm-cov --workspace

# 6. 安全审计
cargo audit

# 7. 文档检查
cargo doc --no-deps --workspace
```

### 自动化工具

```bash
# 安装 pre-commit hook
cat << 'EOF' > .git/hooks/pre-commit
#!/bin/bash
set -e

echo "Running pre-commit checks..."

# Format check
cargo fmt -- --check
if [ $? -ne 0 ]; then
    echo "❌ Format check failed. Run 'cargo fmt' to fix."
    exit 1
fi

# Clippy check
cargo clippy -- -D warnings
if [ $? -ne 0 ]; then
    echo "❌ Clippy check failed. Fix the warnings."
    exit 1
fi

# Tests
cargo test --workspace
if [ $? -ne 0 ]; then
    echo "❌ Tests failed. Fix the tests."
    exit 1
fi

echo "✅ All pre-commit checks passed!"
EOF

chmod +x .git/hooks/pre-commit
```

---

## 📊 开发指标

### 代码质量指标

| 指标 | 目标 | 当前状态 |
|------|------|----------|
| **代码覆盖率** | ≥ 80% | 85% ✅ |
| **Clippy 警告数** | 0 | 0 ✅ |
| **安全漏洞** | 0 | 0 ✅ |
| **文档覆盖率** | ≥ 90% | 95% ✅ |
| **API 文档完整性** | 100% | 100% ✅ |

### 开发效率指标

| 指标 | 平均值 |
|------|--------|
| PR 审查时间 | 4 小时 |
| PR 合并时间 | 24 小时 |
| 测试执行时间 | 3.5 分钟 |
| 构建时间 | 8 分钟 |

---

## 🛠️ 开发工具推荐

### VS Code 扩展

```json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "tamasfe.even-better-toml",
    "serayuzgur.crates",
    "vadimcn.vscode-lldb",
    "ms-azuretools.vscode-docker",
    "ms-vscode.makefile-tools",
    "streetsidesoftware.code-spell-checker"
  ]
}
```

### Cargo 插件

```bash
# 安装常用 cargo 插件
cargo install cargo-watch      # 文件监听自动重新编译
cargo install cargo-edit       # 管理依赖
cargo install cargo-outdated   # 检查过期依赖
cargo install cargo-tree       # 依赖树
cargo install cargo-llvm-cov   # 代码覆盖率
cargo install cargo-audit      # 安全审计
```

---

## 🔗 相关文档

- **快速开始**: [00-quickstart/QUICK_START.md](../00-quickstart/QUICK_START.md)
- **系统架构**: [01-architecture/MULTI_CHAIN_WALLET_ARCHITECTURE.md](../01-architecture/MULTI_CHAIN_WALLET_ARCHITECTURE.md)
- **API 参考**: [03-api/API_REFERENCE.md](../03-api/API_REFERENCE.md)
- **测试指南**: [04-testing/API_TESTING.md](../04-testing/API_TESTING.md)

---

**最后更新**: 2025-12-06  
**维护者**: Development Team  
**审查者**: Tech Lead, Senior Engineers
