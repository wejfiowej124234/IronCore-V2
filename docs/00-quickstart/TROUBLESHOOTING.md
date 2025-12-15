# 🔧 故障排查手册

> 遇到问题？按这份手册一步步诊断！

## 📋 目录

- [诊断流程图](#诊断流程图)
- [服务启动问题](#服务启动问题)
- [数据库连接问题](#数据库连接问题)
- [API调用问题](#api调用问题)
- [区块链网络问题](#区块链网络问题)
- [性能问题](#性能问题)
- [日志分析](#日志分析)
- [紧急联系](#紧急联系)

---

## 诊断流程图

```
问题发生
    ↓
第1步: 查看日志 (80%问题在这里找到答案)
    ↓
第2步: 检查网络连接 (数据库、Redis、RPC)
    ↓
第3步: 验证配置文件 (config.toml)
    ↓
第4步: 检查资源使用 (CPU、内存、磁盘)
    ↓
第5步: 联系技术支持
```

---

## 服务启动问题

### 🔴 问题1: `cargo run` 编译失败

**症状**:
```
error: could not compile `ironforge_backend` due to 3 previous errors
```

**诊断步骤**:

1. **检查Rust版本**
   ```bash
   rustc --version
   # 需要: rustc 1.75.0 或更高
   
   # 更新Rust
   rustup update stable
   ```

2. **清理缓存重新编译**
   ```bash
   cargo clean
   cargo build
   ```

3. **检查依赖冲突**
   ```bash
   cargo tree | grep 依赖包名
   cargo update
   ```

4. **查看详细错误**
   ```bash
   cargo build --verbose
   ```

**常见错误**:

**错误**: `use of unstable library feature`
```bash
# 解决: 切换到stable版本
rustup default stable
```

**错误**: `failed to fetch`
```bash
# 解决: 使用国内镜像
# 编辑 ~/.cargo/config.toml
[source.crates-io]
replace-with = 'ustc'
[source.ustc]
registry = "https://mirrors.ustc.edu.cn/crates.io-index"
```

### 🔴 问题2: 启动后立即退出

**症状**:
```bash
cargo run
# 启动后没有任何输出就退出了
```

**诊断步骤**:

1. **查看日志**
   ```bash
   # 日志位置
   cat backend/debug.log
   tail -f backend/backend.log
   ```

2. **检查配置文件**
   ```bash
   # 确认 config.toml 存在
   ls backend/config.toml
   
   # 验证语法
   cat backend/config.toml
   ```

3. **手动运行看错误**
   ```bash
   cd backend
   RUST_LOG=debug cargo run
   ```

**常见原因**:

**原因1**: `config.toml` 不存在
```bash
# 解决
cd backend
cp config.example.toml config.toml
```

**原因2**: 数据库连接失败（非降级模式）
```toml
# 解决: 启用降级模式
[server]
allow_degraded_start = true
```

**原因3**: 端口被占用
```bash
# Windows
netstat -ano | findstr 8088
taskkill /PID <进程ID> /F

# Linux/Mac
lsof -ti:8088 | xargs kill -9
```

### 🔴 问题3: "panic at 'called `Result::unwrap()` on an `Err` value'"

**症状**:
```
thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: ...'
```

**诊断步骤**:

1. **查看完整错误**
   ```bash
   RUST_BACKTRACE=full cargo run
   ```

2. **常见panic原因**:
   - 环境变量缺失
   - 配置文件格式错误
   - 必需的依赖服务未启动

3. **检查环境变量**
   ```bash
   # 检查关键变量
   echo $DATABASE_URL
   echo $JWT_SECRET
   ```

**解决方案**:
```bash
# 设置完整环境变量
export DATABASE_URL="postgres://root@localhost:26257/ironcore?sslmode=disable"
export REDIS_URL="redis://localhost:6379"
export JWT_SECRET="dev-jwt-secret-min-32-chars-long-xxxxx"
```

---

## 数据库连接问题

### 🔴 问题4: "Connection refused (os error 111)"

**症状**:
```
Error: Connection refused (os error 111)
Database connection failed: could not connect to server
```

**诊断步骤**:

1. **检查数据库是否运行**
   ```bash
   # CockroachDB
   docker ps | grep cockroachdb
   
   # 或直接连接测试
   psql "postgres://root@localhost:26257/defaultdb?sslmode=disable"
   ```

2. **检查端口是否开放**
   ```bash
   telnet localhost 26257
   # 或
   nc -zv localhost 26257
   ```

3. **查看Docker日志**
   ```bash
   docker logs cockroachdb
   ```

**解决方案**:

**方案1**: 启动数据库
```bash
cd ops
docker compose up -d
# 等待30秒让数据库完全启动
sleep 30
```

**方案2**: 降级启动（无数据库）
```toml
# config.toml
[server]
allow_degraded_start = true
```

**方案3**: 检查防火墙
```bash
# Windows
netsh advfirewall firewall add rule name="CockroachDB" dir=in action=allow protocol=TCP localport=26257

# Linux
sudo ufw allow 26257
```

### 🔴 问题5: "password authentication failed"

**症状**:
```
Error: password authentication failed for user "root"
```

**诊断步骤**:

1. **检查DATABASE_URL**
   ```bash
   echo $DATABASE_URL
   # 应该是: postgres://root@localhost:26257/ironcore?sslmode=disable
   # 注意: CockroachDB root用户默认无密码
   ```

2. **测试连接**
   ```bash
   psql "$DATABASE_URL" -c "SELECT version();"
   ```

**解决方案**:
```bash
# 正确的连接字符串（无密码）
export DATABASE_URL="postgres://root@localhost:26257/ironcore?sslmode=disable"
```

### 🔴 问题6: "database does not exist"

**症状**:
```
Error: database "ironcore" does not exist
```

**解决方案**:

1. **创建数据库**
   ```bash
   # 连接到 defaultdb
   psql "postgres://root@localhost:26257/defaultdb?sslmode=disable"
   
   # 创建数据库
   CREATE DATABASE ironcore;
   \q
   ```

2. **运行迁移**
   ```bash
   cd backend
   export DATABASE_URL="postgres://root@localhost:26257/ironcore?sslmode=disable"
   sqlx migrate run
   ```

---

## API调用问题

### 🔴 问题7: "404 Not Found"

**症状**:
```bash
curl http://localhost:8088/api/wallet/create
# {"error":"Not Found"}
```

**诊断步骤**:

1. **检查URL拼写**
   ```bash
   # ❌ 错误
   /api/wallet/create
   
   # ✅ 正确
   /api/wallets/create
   ```

2. **查看路由列表**
   ```bash
   # 查看文档
   cat backend/docs/01-architecture/API_ROUTES_MAP.md
   
   # 或查看代码
   grep -r "route" backend/src/api/mod.rs
   ```

3. **确认服务版本**
   ```bash
   curl http://localhost:8088/api/health
   ```

**解决方案**: 使用正确的API路径

### 🔴 问题8: "CORS policy" 错误

**症状**（浏览器控制台）:
```
Access to fetch at 'http://localhost:8088/api/wallets' 
from origin 'http://localhost:3000' has been blocked by CORS policy
```

**诊断步骤**:

1. **检查CORS配置**
   ```bash
   # 查看配置
   grep -A5 "CORS" backend/src/main.rs
   ```

2. **检查请求来源**
   ```javascript
   // 浏览器控制台
   console.log(window.location.origin);
   ```

**解决方案**:

1. **添加 CORS 配置**
   ```rust
   // backend/src/main.rs
   use tower_http::cors::{CorsLayer, Any};
   
   let cors = CorsLayer::new()
       .allow_origin(Any)
       .allow_methods(Any)
       .allow_headers(Any);
   
   let app = Router::new()
       .route(...)
       .layer(cors);
   ```

2. **或使用代理**（开发环境）
   ```javascript
   // frontend/vite.config.js
   export default {
     server: {
       proxy: {
         '/api': 'http://localhost:8088'
       }
     }
   }
   ```

### 🔴 问题9: "429 Too Many Requests"

**症状**:
```json
{
  "error": "RateLimitExceeded",
  "message": "Rate limit exceeded: 100 requests per minute",
  "retry_after": 60
}
```

**诊断步骤**:

1. **检查请求频率**
   ```bash
   # 查看日志中的请求时间戳
   grep "POST /api" backend/debug.log | tail -20
   ```

2. **确认是否在循环中调用**
   ```javascript
   // ❌ 错误
   while (true) {
     await fetch('/api/wallets');
   }
   ```

**解决方案**:

1. **实现退避重试**
   ```javascript
   async function apiCall(url, retries = 3) {
     for (let i = 0; i < retries; i++) {
       const res = await fetch(url);
       if (res.status === 429) {
         const retryAfter = res.headers.get('Retry-After') || 60;
         await sleep(retryAfter * 1000);
         continue;
       }
       return res;
     }
   }
   ```

2. **增加限流阈值**（管理员）
   ```toml
   # config.toml
   [server]
   rate_limit_per_minute = 200  # 默认100
   ```

---

## 区块链网络问题

### 🔴 问题10: "RPC endpoint unreachable"

**症状**:
```
Error: Failed to connect to Ethereum RPC: https://mainnet.infura.io/v3/...
```

**诊断步骤**:

1. **测试RPC连接**
   ```bash
   curl -X POST https://mainnet.infura.io/v3/YOUR_KEY \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
   ```

2. **检查RPC配置**
   ```bash
   # 查看配置的RPC端点
   grep -r "rpc_url" backend/src/
   ```

3. **检查网络**
   ```bash
   ping mainnet.infura.io
   ```

**解决方案**:

1. **更换RPC提供商**
   ```rust
   // 使用备用RPC
   const FALLBACK_RPCS: &[&str] = &[
       "https://mainnet.infura.io/v3/KEY",
       "https://eth-mainnet.alchemyapi.io/v2/KEY",
       "https://cloudflare-eth.com",
   ];
   ```

2. **使用本地节点**
   ```bash
   # 启动Geth
   geth --http --http.addr 0.0.0.0 --http.port 8545
   
   # 配置使用本地
   export ETH_RPC_URL="http://localhost:8545"
   ```

### 🔴 问题11: "Gas estimation failed"

**症状**:
```
Error: Execution reverted: insufficient funds for gas
```

**诊断步骤**:

1. **检查账户余额**
   ```bash
   curl "http://localhost:8088/api/asset/balance?chain=ethereum&address=0x..."
   ```

2. **检查Gas价格**
   ```bash
   curl "http://localhost:8088/api/gas/price?chain=ethereum"
   ```

3. **手动估算Gas**
   ```bash
   curl -X POST http://localhost:8088/api/gas/estimate \
     -H "Content-Type: application/json" \
     -d '{
       "chain": "ethereum",
       "from": "0x...",
       "to": "0x...",
       "value": "0.1"
     }'
   ```

**解决方案**:

1. **确保有足够余额**
   ```
   需要: 转账金额 + Gas费
   例如: 0.1 ETH + 0.001 ETH = 0.101 ETH
   ```

2. **降低Gas价格**（会延长确认时间）
   ```javascript
   const gasPrice = await getGasPrice('slow');
   ```

---

## 性能问题

### 🔴 问题12: API响应很慢（>5秒）

**诊断步骤**:

1. **测量响应时间**
   ```bash
   curl -w "@curl-format.txt" http://localhost:8088/api/wallets
   
   # curl-format.txt:
   # time_total: %{time_total}s\n
   ```

2. **检查数据库查询**
   ```bash
   # 启用查询日志
   export SQLX_LOGGING=trace
   cargo run
   ```

3. **检查资源使用**
   ```bash
   # CPU
   top -p $(pgrep ironforge_backend)
   
   # 内存
   ps aux | grep ironforge_backend
   
   # 数据库连接
   docker exec cockroachdb cockroach sql --insecure -e "SHOW SESSIONS;"
   ```

**解决方案**:

1. **启用Redis缓存**
   ```toml
   [redis]
   url = "redis://localhost:6379"
   cache_ttl_secs = 300
   ```

2. **增加数据库连接池**
   ```toml
   [database]
   max_connections = 50  # 默认20
   ```

3. **添加索引**
   ```sql
   CREATE INDEX idx_wallets_address ON wallets(address);
   CREATE INDEX idx_transactions_hash ON transactions(tx_hash);
   ```

### 🔴 问题13: 内存泄漏

**症状**:
```bash
# 内存持续增长
watch -n 1 'ps aux | grep ironforge_backend | grep -v grep'
```

**诊断步骤**:

1. **使用 valgrind**
   ```bash
   cargo build
   valgrind --leak-check=full ./target/debug/ironforge_backend
   ```

2. **检查未关闭的连接**
   ```rust
   // 查找未 drop 的资源
   grep -r "new(" backend/src/ | grep -v "drop"
   ```

**解决方案**:
- 确保所有数据库连接正确关闭
- 使用 `Arc` 而不是 `Box` 共享数据
- 定期重启服务（临时方案）

---

## 日志分析

### 日志位置

| 日志类型 | 路径 | 用途 |
|---------|------|------|
| 应用日志 | `backend/debug.log` | 所有运行时日志 |
| 错误日志 | `backend/error.log` | 仅错误信息 |
| 数据库日志 | Docker容器内 | 数据库查询日志 |
| 访问日志 | `backend/access.log` | HTTP请求日志 |

### 常用日志分析命令

```bash
# 查看最新100行
tail -n 100 backend/debug.log

# 实时查看
tail -f backend/debug.log

# 搜索错误
grep "ERROR" backend/debug.log

# 统计API调用次数
grep "POST /api" backend/debug.log | wc -l

# 查看慢查询
grep "SLOW QUERY" backend/debug.log

# 按时间过滤
grep "2025-11-24T10:" backend/debug.log

# 导出特定时间段日志
sed -n '/2025-11-24T10:00/,/2025-11-24T11:00/p' backend/debug.log > problem.log
```

### 日志级别

| 级别 | 用途 | 示例 |
|------|------|------|
| **TRACE** | 非常详细 | 函数调用、变量值 |
| **DEBUG** | 调试信息 | SQL查询、中间结果 |
| **INFO** | 普通信息 | 服务启动、请求完成 |
| **WARN** | 警告 | 连接重试、降级模式 |
| **ERROR** | 错误 | 请求失败、数据库错误 |

---

## 紧急联系

### 联系流程

```
1. 自助诊断（本手册）
   ↓ 未解决
2. 查看FAQ
   ↓ 未解决
3. 技术群求助
   ↓ 未解决
4. 提交Issue
   ↓ 紧急问题
5. 联系On-Call工程师
```

### 提Issue模板

```markdown
### 问题描述
[简要描述问题]

### 复现步骤
1. 第一步
2. 第二步
3. 观察到的问题

### 期望结果
[应该是什么样]

### 实际结果
[实际是什么样]

### 环境信息
- OS: [Windows 11 / Ubuntu 22.04]
- Rust版本: `rustc --version`
- 后端版本: [0.1.0]
- 数据库: [CockroachDB v23.1]

### 日志
```
[粘贴相关日志]
```

### 已尝试的解决方案
- [ ] 检查了日志
- [ ] 重启了服务
- [ ] 清理了缓存
```

---

## 附录：健康检查清单

### 每日检查（自动化）

- [ ] 服务是否运行: `curl http://localhost:8088/api/health`
- [ ] 数据库连接: `psql $DATABASE_URL -c "SELECT 1;"`
- [ ] Redis连接: `redis-cli ping`
- [ ] 磁盘空间: `df -h`
- [ ] 内存使用: `free -h`

### 每周检查（手动）

- [ ] 日志文件大小: `du -sh backend/*.log`
- [ ] 数据库大小: `SELECT pg_size_pretty(pg_database_size('ironcore'));`
- [ ] 审查错误日志: `grep ERROR backend/error.log | wc -l`
- [ ] 性能测试: `ab -n 1000 -c 10 http://localhost:8088/api/health`

### 每月检查

- [ ] 依赖更新: `cargo outdated`
- [ ] 安全审计: `cargo audit`
- [ ] 数据库备份验证
- [ ] 性能基准对比

---

**最后更新**: 2025-11-24  
**维护者**: Backend Team  
**紧急联系**: oncall@example.com
