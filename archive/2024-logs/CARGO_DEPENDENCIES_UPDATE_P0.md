# Cargo依赖更新清单（P0修复）

## 需要添加的依赖

```bash
cd IronCore

# Redis（分布式锁 + 缓存）
cargo add redis --features tokio-comp,connection-manager

# 正则表达式（日志脱敏）
cargo add regex

# 验证编译
cargo check
```

## 完整Cargo.toml（确认包含以下依赖）

```toml
[dependencies]
# Redis（新增 - P0-1）
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }

# 正则表达式（新增 - P0-7）
regex = "1"

# 其他依赖保持不变...
```

## 验证

```bash
# 检查依赖
cargo tree | grep redis
cargo tree | grep regex

# 编译测试
cargo test --lib
```

