# 🚀 快速启动指南

## 当前目录问题

如果你在 `IronCore/ops` 目录，需要先回到 `IronCore` 目录：

```bash
cd ..
# 现在你在 IronCore 目录了
```

## 启动后端

### 方法 1: 使用快速启动脚本（推荐）

**Git Bash**:
```bash
cd IronCore
chmod +x start-backend.sh
./start-backend.sh
```

**Windows CMD**:
```bash
cd IronCore
start-backend.bat
```

### 方法 2: 手动启动

**Git Bash**:
```bash
cd IronCore
export WALLET_ENC_KEY="dev-wallet-encryption-key-32chars!!"
cargo run --profile release-fast
```

**PowerShell**:
```powershell
cd IronCore
$env:WALLET_ENC_KEY="dev-wallet-encryption-key-32chars!!"
cargo run --profile release-fast
```

### 方法 3: 使用完整启动脚本

从项目根目录运行：
```bash
cd ~/Desktop/Rust-Blockchain
Start-Production-Stack.bat
```

---

## 📍 目录结构

```
Rust-Blockchain/
├── IronCore/              ← 后端代码在这里
│   ├── start-backend.sh   ← 快速启动脚本
│   ├── start-backend.bat  ← Windows 启动脚本
│   ├── config.toml        ← 配置文件
│   └── src/
├── ops/                   ← Docker 配置
└── Start-Production-Stack.bat  ← 完整启动脚本
```

---

## ✅ 检查清单

启动前：
- [ ] 在正确的目录（`IronCore`）
- [ ] `config.toml` 存在
- [ ] Docker 服务运行中（CockroachDB, Redis, ImmuDB）
- [ ] `WALLET_ENC_KEY` 已设置

---

## 🔧 常见问题

### 问题 1: "No such file or directory"

**原因**: 不在正确的目录

**解决**:
```bash
# 检查当前位置
pwd

# 回到 IronCore 目录
cd ~/Desktop/Rust-Blockchain/IronCore

# 或从当前目录
cd ../IronCore
```

### 问题 2: "Permission denied"

**解决**:
```bash
chmod +x start-backend.sh
```

### 问题 3: WALLET_ENC_KEY 错误

**解决**:
```bash
export WALLET_ENC_KEY="dev-wallet-encryption-key-32chars!!"
```

---

## 🚀 现在可以启动

```bash
# 确保在 IronCore 目录
cd ~/Desktop/Rust-Blockchain/IronCore

# 运行启动脚本
./start-backend.sh
```

