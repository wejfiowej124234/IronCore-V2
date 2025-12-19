# 🚀 最简单的启动方法

## ✅ 方法 1: 使用最简单的脚本（推荐）

### Windows
双击运行：
```
IronCore-V2/run.bat
```

或在命令行：
```bash
cd IronCore-V2
run.bat
```

### Git Bash/Linux/Mac
```bash
cd IronCore-V2
chmod +x run.sh
./run.sh
```

---

## ✅ 方法 2: 直接运行命令（最可靠）

### Windows CMD
```cmd
cd C:\Users\plant\Desktop\Rust-Blockchain\IronCore-V2
set WALLET_ENC_KEY=dev-wallet-encryption-key-32chars!!
cargo run --profile release-fast
```

### Windows PowerShell
```powershell
cd C:\Users\plant\Desktop\Rust-Blockchain\IronCore-V2
$env:WALLET_ENC_KEY="dev-wallet-encryption-key-32chars!!"
cargo run --profile release-fast
```

### Git Bash
```bash
cd ~/Desktop/Rust-Blockchain/IronCore-V2
export WALLET_ENC_KEY="dev-wallet-encryption-key-32chars!!"
cargo run --profile release-fast
```

---

## 🔍 如果还是启动不了

### 检查 1: 确认在正确目录
```bash
# 应该看到 config.toml
ls config.toml

# 应该看到 Cargo.toml
ls Cargo.toml
```

### 检查 2: 确认 Docker 运行
```bash
docker ps --filter "name=cockroachdb"
```

### 检查 3: 检查 Rust 工具链
```bash
rustc --version
cargo --version
```

### 检查 4: 查看详细错误
直接运行 cargo，查看完整错误信息：
```bash
cd IronCore-V2
cargo run --profile release-fast 2>&1 | tee error.log
```

---

## 🆘 常见错误

### 错误 1: "WALLET_ENC_KEY is required"
**解决**: 确保设置了环境变量
```bash
export WALLET_ENC_KEY="dev-wallet-encryption-key-32chars!!"
```

### 错误 2: "config.toml not found"
**解决**: 确保在 IronCore 目录
```bash
cd ~/Desktop/Rust-Blockchain/IronCore-V2
pwd  # 应该显示 .../IronCore-V2
```

### 错误 3: "Database connection failed"
**解决**: 启动 Docker 服务
```bash
cd ~/Desktop/Rust-Blockchain/ops
docker compose up -d
```

---

## 📋 完整启动流程

```bash
# 1. 启动 Docker（如果未运行）
cd ~/Desktop/Rust-Blockchain/ops
docker compose up -d

# 2. 等待服务就绪
sleep 10

# 3. 启动后端
cd ~/Desktop/Rust-Blockchain/IronCore-V2
export WALLET_ENC_KEY="dev-wallet-encryption-key-32chars!!"
cargo run --profile release-fast
```

---

## ✅ 最简单的测试

直接运行这个命令（在 IronCore 目录）：

**Git Bash**:
```bash
cd ~/Desktop/Rust-Blockchain/IronCore
WALLET_ENC_KEY="dev-wallet-encryption-key-32chars!!" cargo run --profile release-fast
```

**PowerShell**:
```powershell
cd C:\Users\plant\Desktop\Rust-Blockchain\IronCore
$env:WALLET_ENC_KEY="dev-wallet-encryption-key-32chars!!"; cargo run --profile release-fast
```

---

**试试最简单的方法：直接运行命令！** 🚀

