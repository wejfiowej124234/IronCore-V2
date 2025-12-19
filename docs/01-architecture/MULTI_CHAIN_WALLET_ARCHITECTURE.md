# 多链钱包架构设计文档

## 📐 架构概览

### 设计原则
1. ✅ **相同曲线共享代码** - secp256k1 的链（ETH/BSC/Polygon/Bitcoin）复用实现
2. ✅ **策略模式分离** - 不同曲线（ed25519/sr25519）独立实现
3. ✅ **统一接口** - 对外提供一致的 API
4. ✅ **链配置驱动** - 通过配置而非硬编码来支持新链

### 架构分层

```
┌────────────────────────────────────────────────────────────────┐
│                     API Layer (统一接口)                        │
│  POST /api/v1/wallets/batch { wallets: [...] }                 │
│  GET  /api/v1/balance { chain, address }                        │
│  POST /api/v1/transactions { signed_tx, ... }                   │
│  (以 /openapi.yaml 与 /docs 为准)                               │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│              Service Layer (业务逻辑)                           │
│  MultiChainWalletService::create_wallet(request)               │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│          Chain Strategy Factory (策略工厂)                      │
│  match chain.curve_type {                                       │
│    Secp256k1 => Secp256k1Strategy,  ← ETH/BSC/Polygon/BTC 共享 │
│    Ed25519   => Ed25519Strategy,    ← Solana/Cardano 共享      │
│    Sr25519   => Sr25519Strategy,    ← Polkadot/Kusama 共享     │
│  }                                                              │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌──────────────────┬──────────────────┬────────────────────────┐
│ Secp256k1Strategy│  Ed25519Strategy │  Sr25519Strategy       │
│ (4+ chains)      │  (2+ chains)     │  (2+ chains)           │
└──────────────────┴──────────────────┴────────────────────────┘
```

---

## 🎯 当前支持的链

### Secp256k1 系列 (共享实现)
| 链 | Chain ID | Symbol | 派生路径 | 地址格式 |
|---|---|---|---|---|
| Ethereum Mainnet | 1 | ETH | m/44'/60'/0'/0/{index} | 0x... (hex) |
| Ethereum Sepolia | 11155111 | ETH | m/44'/60'/0'/0/{index} | 0x... (hex) |
| BSC | 56 | BNB | m/44'/60'/0'/0/{index} | 0x... (hex) |
| Polygon | 137 | MATIC | m/44'/60'/0'/0/{index} | 0x... (hex) |
| Bitcoin | 0 | BTC | m/84'/0'/0'/0/{index} | bc1... (bech32) |

### Ed25519 系列 (独立实现)
| 链 | Chain ID | Symbol | 派生路径 | 地址格式 |
|---|---|---|---|---|
| Solana | 501 | SOL | m/44'/501'/0'/0' | Base58 (32-44 chars) |
| Cardano | 1815 | ADA | m/1852'/1815'/0'/0/{index} | addr1... (bech32) |

### Sr25519 系列 (待实现)
| 链 | Chain ID | Symbol | 派生路径 | 地址格式 |
|---|---|---|---|---|
| Polkadot | 354 | DOT | m/44'/354'/0'/0'/{index} | SS58 |

---

## 💻 使用示例（非托管：客户端派生 + 后端登记）

> 关键原则：助记词/私钥只存在于客户端本地；后端只接收地址、公钥等公开信息，以及已签名交易。
> 具体端点与认证要求以 `/openapi.yaml` 与 `/docs` 为准。

### 1. 客户端本地派生地址（示意）

- 客户端生成助记词与私钥（BIP39/BIP44 等）
- 客户端按链的派生路径得到 `address` 与 `public_key`

### 2. 批量登记钱包到后端

```bash
curl -X POST http://localhost:8088/api/v1/wallets/batch \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_token>" \
  -d '{
    "wallets": [
      {
        "chain": "ethereum",
        "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb6",
        "public_key": "0x...",
        "name": "Main Wallet"
      }
    ]
  }'
```

### 3. 查询余额（示意）

```bash
curl "http://localhost:8088/api/v1/balance?chain=ethereum&address=0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb6"
```

---

---

## 🚀 如何添加新链

### 步骤 1: 在 `ChainRegistry` 中注册链

编辑 `IronCore-V2/src/domain/chain_config.rs`:

```rust
// Avalanche C-Chain
self.register(ChainConfig {
    chain_id: 43114,
    name: "Avalanche C-Chain".to_string(),
    symbol: "AVAX".to_string(),
    curve_type: CurveType::Secp256k1,  // ✅ 复用 secp256k1
    address_format: AddressFormat::Hex,
    derivation_standard: DerivationStandard::BIP44,
    coin_type: 60, // 使用 ETH 兼容路径
    derivation_path_template: "m/44'/60'/0'/0/{index}".to_string(),
    is_testnet: false,
    rpc_url: Some("https://api.avax.network/ext/bc/C/rpc".to_string()),
});
```

### 步骤 2: 如果是新曲线类型，实现 DerivationStrategy

编辑 `IronCore-V2/src/domain/derivation.rs`:

```rust
pub struct NewCurveStrategy;

impl DerivationStrategy for NewCurveStrategy {
    fn derive_wallet(...) -> Result<DerivedWallet> {
        // 实现派生逻辑
    }
    
    fn validate_address(...) -> Result<bool> {
        // 实现地址验证
    }
}

// 在工厂中注册
impl DerivationStrategyFactory {
    pub fn create_strategy(curve_type: CurveType) -> Box<dyn DerivationStrategy> {
        match curve_type {
            CurveType::NewCurve => Box::new(NewCurveStrategy),
            ...
        }
    }
}
```

### 步骤 3: 测试

```rust
#[test]
fn test_new_chain() {
    let service = MultiChainWalletService::new();
    let request = CreateWalletRequest {
        chain: "AVAX".to_string(),
        ...
    };
    let response = service.create_wallet(request).unwrap();
    assert_eq!(response.chain.symbol, "AVAX");
}
```

---

## 🎓 核心优势

### 1. 代码复用
- ✅ Ethereum/BSC/Polygon 共享 90% 代码
- ✅ 新增 EVM 兼容链只需配置，无需编码

### 2. 类型安全
- ✅ Rust 类型系统保证曲线不会混淆
- ✅ 编译时检查派生路径合法性

### 3. 易于扩展
- ✅ 新增链：修改 `ChainRegistry`
- ✅ 新增曲线：实现 `DerivationStrategy` trait

### 4. 统一接口
- ✅ 所有链使用相同的 API
- ✅ 前端无需关心曲线细节

---

## 📦 依赖项

需要在 `Cargo.toml` 添加：

```toml
[dependencies]
# 加密曲线
k256 = { version = "0.13", features = ["ecdsa", "sha256"] }
ed25519-dalek = "2.1"
# schnorrkel = "0.11"  # sr25519 (Polkadot)

# BIP 标准
bip39 = "2.2"
coins-bip32 = "0.8"

# 编码
hex = "0.4"
bs58 = "0.5"
sha3 = "0.10"

# Web 框架
axum = "0.7"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
```

---

## ⚠️ 安全注意事项

1. **私钥管理**
   - ⚠️ 私钥应加密存储（使用 AES-256-GCM）
   - ⚠️ API 不应返回私钥给客户端
   - ✅ 使用 `zeroize` 清除内存中的敏感数据

2. **助记词处理**
   - ⚠️ 助记词仅在创建时返回一次
   - ✅ 使用 HTTPS 传输
   - ✅ 建议客户端立即加密存储

3. **地址验证**
   - ✅ 发送交易前必须验证地址格式
   - ✅ 使用 `validate_address` 端点

---

## 🔮 未来扩展

### 短期 (1-2周)
- [ ] 实现 Sr25519 策略 (Polkadot/Kusama)
- [ ] 完善 Bitcoin 地址生成 (使用 `bitcoin` crate)
- [ ] 添加 Cardano 完整支持 (使用 `cardano-serialization-lib`)

### 中期 (1个月)
- [ ] 支持更多 EVM 链 (Arbitrum, Optimism, Avalanche)
- [ ] 支持 Cosmos 生态 (ATOM, OSMO, JUNO)
- [ ] 添加硬件钱包支持

### 长期 (3个月+)
- [ ] 多签钱包支持
- [ ] MPC (Multi-Party Computation) 集成
- [ ] 量子安全算法研究

---

## 📚 参考资料

- [BIP39 - Mnemonic Code](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki)
- [BIP44 - Multi-Account Hierarchy](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki)
- [SLIP-0010 - Universal Derivation](https://github.com/satoshilabs/slips/blob/master/slip-0010.md)
- [EIP-155 - Chain IDs](https://eips.ethereum.org/EIPS/eip-155)
- [Solana Derivation Path](https://docs.solana.com/wallet-guide/paper-wallet#hierarchical-derivation)
