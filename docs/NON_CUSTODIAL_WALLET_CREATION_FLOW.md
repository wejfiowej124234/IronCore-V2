# 非托管钱包创建流程（企业级标准）

**更新日期**: 2025-12-02  
**安全等级**: 🔴 Production-Grade Non-Custodial  
**架构类型**: Pure Non-Custodial Wallet

---

## 🔐 核心原则

### ✅ 非托管钱包的本质

```
用户 100% 控制私钥 = 用户 100% 控制资产
后端 0% 接触私钥 = 后端 0% 能动用资产
```

**关键声明**:
- ❌ 后端不能存储私钥、助记词、种子
- ❌ 后端不能解密用户密钥
- ❌ 后端不能代替用户签名交易
- ✅ 后端只存储公开地址和元数据
- ✅ 用户自己负责备份助记词
- ✅ 助记词丢失 = 资产永久丢失（不可恢复）

---

## 🎯 完整钱包创建流程

### Step 1: 前端生成助记词（100%本地）

```typescript
// IronForge/src/features/wallet/create.tsx

async function createWallet() {
  // 1.1 生成随机熵（使用OS级随机数生成器）
  const entropy = crypto.getRandomValues(new Uint8Array(32)); // 256 bits
  
  // 1.2 生成BIP39助记词（24个单词）
  const mnemonic = generateMnemonic(entropy, 24);
  // 示例: "abandon ability able about above absent absorb abstract absurd abuse access accident ..."
  
  // 1.3 显示助记词给用户（仅此一次！）
  showMnemonicBackupUI({
    mnemonic,
    warning: "⚠️ 请妥善保管！这是恢复钱包的唯一方式",
    instructions: [
      "1. 手写抄录到纸上",
      "2. 存放到安全地方（保险柜）",
      "3. 不要截图或拍照",
      "4. 不要通过网络传输",
      "5. 确认已备份后点击'我已备份'按钮"
    ]
  });
  
  return mnemonic;
}
```

---

### Step 2: 前端派生多链地址（100%本地）

```typescript
async function deriveAddresses(mnemonic: string): Promise<WalletAddresses> {
  // 2.1 BIP39: 助记词 → 种子
  const seed = mnemonicToSeed(mnemonic); // 512 bits seed
  
  // 2.2 BIP32/BIP44: 派生多链地址
  const addresses = {
    // EVM链（使用secp256k1）
    ETH: deriveAddress(seed, "m/44'/60'/0'/0/0"),   // Ethereum
    BSC: deriveAddress(seed, "m/44'/60'/0'/0/0"),   // 同ETH路径
    POLYGON: deriveAddress(seed, "m/44'/60'/0'/0/0"), // 同ETH路径
    
    // Bitcoin（使用secp256k1，Native SegWit）
    BTC: deriveAddress(seed, "m/84'/0'/0'/0/0"),
    
    // Solana（使用ed25519）
    SOL: deriveAddress(seed, "m/44'/501'/0'/0'"),
    
    // TON（使用ed25519）
    TON: deriveAddress(seed, "m/44'/607'/0'/0'/0'/0'"),
  };
  
  // 2.3 同时派生公钥（用于后端存储）
  const publicKeys = {
    ETH: derivePublicKey(seed, "m/44'/60'/0'/0/0"),
    BSC: derivePublicKey(seed, "m/44'/60'/0'/0/0"),
    // ...
  };
  
  return { addresses, publicKeys };
}

function deriveAddress(seed: Uint8Array, path: string): string {
  const hdWallet = HDKey.fromMasterSeed(seed);
  const child = hdWallet.derive(path);
  const privateKey = child.privateKey;
  
  // 根据不同链生成地址
  if (path.includes("60")) { // EVM
    const address = privateKeyToEthAddress(privateKey);
    privateKey.fill(0); // ✅ 立即清零私钥
    return address;
  } else if (path.includes("0")) { // Bitcoin
    const address = privateKeyToBtcAddress(privateKey);
    privateKey.fill(0); // ✅ 立即清零私钥
    return address;
  }
  // ...
}
```

---

### Step 3: 前端加密助记词（100%本地）

```typescript
async function encryptMnemonicLocally(
  mnemonic: string, 
  walletPassword: string
): Promise<EncryptedMnemonic> {
  // 3.1 生成随机盐（32字节）
  const salt = crypto.getRandomValues(new Uint8Array(32));
  
  // 3.2 使用PBKDF2派生加密密钥（600,000迭代）
  const encryptionKey = await crypto.subtle.deriveKey(
    {
      name: "PBKDF2",
      salt: salt,
      iterations: 600_000, // OWASP 2023标准
      hash: "SHA-256"
    },
    await crypto.subtle.importKey(
      "raw",
      new TextEncoder().encode(walletPassword),
      "PBKDF2",
      false,
      ["deriveKey"]
    ),
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt"]
  );
  
  // 3.3 生成随机IV（12字节，GCM标准）
  const iv = crypto.getRandomValues(new Uint8Array(12));
  
  // 3.4 AES-256-GCM加密助记词
  const ciphertext = await crypto.subtle.encrypt(
    { name: "AES-GCM", iv: iv },
    encryptionKey,
    new TextEncoder().encode(mnemonic)
  );
  
  return {
    ciphertext: base64Encode(ciphertext),
    salt: base64Encode(salt),
    iv: base64Encode(iv),
    algorithm: "AES-256-GCM",
    iterations: 600_000
  };
}
```

---

### Step 4: 前端存储到IndexedDB（100%本地）

```typescript
async function saveToIndexedDB(
  walletName: string,
  encryptedMnemonic: EncryptedMnemonic,
  addresses: WalletAddresses,
  publicKeys: Record<string, string>
): Promise<void> {
  const db = await openDB("ironforge_wallets", 2);
  
  const walletData = {
    id: generateWalletId(addresses),
    name: walletName,
    encryptedMnemonic, // ✅ 加密的助记词
    addresses,         // ✅ 公开地址（可存储）
    publicKeys,        // ✅ 公钥（可存储）
    createdAt: Date.now(),
    version: 2
  };
  
  await db.put("wallets", walletData);
  
  console.log("✅ 钱包已安全存储到本地IndexedDB");
  console.log("❌ 助记词已加密，不会上传到服务器");
}
```

---

### Step 5: 前端发送公开信息到后端

```typescript
async function registerWalletWithBackend(
  addresses: WalletAddresses,
  publicKeys: Record<string, string>
): Promise<void> {
  // 5.1 为每条链创建钱包记录
  const requests = Object.keys(addresses).map(chain => ({
    chain,
    address: addresses[chain],
    public_key: publicKeys[chain],
    derivation_path: DERIVATION_PATHS[chain], // 公开信息
    curve_type: CURVE_TYPES[chain]            // 公开信息
  }));
  
  // 5.2 批量发送到后端（仅公开信息）
  const response = await fetch("/api/wallets/batch-create", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "Authorization": `Bearer ${jwt_token}`
    },
    body: JSON.stringify({
      wallets: requests
      // ❌ 不发送: mnemonic, private_key, wallet_password
    })
  });
  
  if (!response.ok) {
    throw new Error("Failed to register wallet with backend");
  }
  
  console.log("✅ 钱包地址已绑定到用户账户");
}
```

---

## 🔄 完整数据流图

```
┌─────────────────────────────────────────────────────────┐
│              前端（IronForge WASM）                      │
│                100% 本地操作                             │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Step 1: 生成助记词                                     │
│    Entropy (256 bits) → BIP39 Mnemonic (24 words)      │
│    ↓                                                    │
│    展示给用户（仅一次）                                 │
│    "abandon ability able about ..."                     │
│                                                         │
│  Step 2: 派生地址                                       │
│    Mnemonic → Seed (512 bits)                           │
│    ↓ BIP32/BIP44                                        │
│    ETH:  0x742d35Cc6634C0532925a3b844Bc9e7595f0bFd2    │
│    BSC:  0x742d35Cc... (同ETH)                         │
│    BTC:  bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh    │
│    SOL:  7S3P4HxJpyyigGzodYwHtCxZyUQe9JiBMHyRWXArAaKv   │
│    TON:  0:5d7e8f9a... (简化格式)                       │
│                                                         │
│  Step 3: 加密助记词                                     │
│    钱包密码 + PBKDF2 (600k) → 加密密钥                  │
│    ↓ AES-256-GCM                                        │
│    加密的助记词（base64）                               │
│                                                         │
│  Step 4: 存储到IndexedDB                                │
│    {                                                    │
│      encryptedMnemonic: "xK9mP2...",  ✅               │
│      addresses: {...},                ✅               │
│      publicKeys: {...}                ✅               │
│    }                                                    │
│                                                         │
└────────────────────┬────────────────────────────────────┘
                     │ HTTPS POST
                     │ /api/wallets/batch-create
                     │
                     │ Body: {
                     │   wallets: [
                     │     {
                     │       chain: "ETH",
                     │       address: "0x742d...",  ✅
                     │       public_key: "0x04...", ✅
                     │       derivation_path: "m/44'/60'/0'/0/0" ✅
                     │     },
                     │     ...
                     │   ]
                     │   // ❌ 不发送: mnemonic, private_key, wallet_password
                     │ }
                     ▼
┌─────────────────────────────────────────────────────────┐
│              后端（IronCore Rust）                       │
│             只存储公开信息                               │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Step 5: 验证并存储                                     │
│    1. 验证JWT token                                     │
│    2. 验证地址格式                                      │
│    3. 验证地址未重复                                    │
│    4. 存储到数据库                                      │
│                                                         │
│  数据库（wallets表）:                                   │
│    ┌─────────────────────────────────────┐            │
│    │ id                  UUID            │            │
│    │ user_id             UUID            │            │
│    │ chain_id            INT             │            │
│    │ address             TEXT  ✅        │            │
│    │ pubkey              TEXT  ✅        │            │
│    │ derivation_path     TEXT  ✅        │            │
│    │ curve_type          TEXT  ✅        │            │
│    │ created_at          TIMESTAMP       │            │
│    │                                     │            │
│    │ ❌ encrypted_private_key (已删除)   │            │
│    │ ❌ encryption_nonce (已删除)        │            │
│    │ ❌ mnemonic (禁止存储)              │            │
│    └─────────────────────────────────────┘            │
│                                                         │
│  返回响应:                                              │
│    {                                                    │
│      "success": true,                                   │
│      "wallets": [                                       │
│        {                                                │
│          "id": "uuid",                                  │
│          "address": "0x742d...",                        │
│          "chain": "ETH"                                 │
│        }                                                │
│      ]                                                  │
│    }                                                    │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

## 🔒 安全保证

### 1. 私钥控制权

| 问题 | 答案 |
|------|------|
| 谁拥有私钥？ | ✅ 100% 用户（通过助记词） |
| 后端能访问私钥吗？ | ❌ 不能，后端没有解密能力 |
| 平台被黑，用户资产安全吗？ | ✅ 安全，后端没有私钥 |
| 用户丢失助记词怎么办？ | ⚠️ 无法恢复（非托管的代价） |

### 2. 密码体系

| 密码类型 | 用途 | 存储位置 | 能否重置 |
|---------|------|---------|---------|
| **账户密码** | 登录后端账户 | 后端（Argon2 hash） | ✅ 可重置 |
| **钱包密码** | 解锁本地钱包 | 不存储（仅派生密钥） | ❌ 不可重置 |

### 3. 数据存储

| 数据类型 | 前端存储 | 后端存储 | 可公开 |
|---------|---------|---------|--------|
| 助记词 | ✅ 加密存储 | ❌ 禁止 | ❌ 绝密 |
| 私钥 | ❌ 不存储 | ❌ 禁止 | ❌ 绝密 |
| 钱包密码 | ❌ 不存储 | ❌ 禁止 | ❌ 绝密 |
| 地址 | ✅ 明文存储 | ✅ 存储 | ✅ 公开 |
| 公钥 | ✅ 明文存储 | ✅ 存储 | ✅ 公开 |

---

## 📝 后端API规范

### POST /api/wallets/batch-create

**请求体** (JSON):
```json
{
  "wallets": [
    {
      "chain": "ETH",
      "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bFd2",
      "public_key": "0x04ab3c8b...",
      "derivation_path": "m/44'/60'/0'/0/0",
      "curve_type": "secp256k1"
    },
    {
      "chain": "BTC",
      "address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
      "public_key": "02f3a8b2...",
      "derivation_path": "m/84'/0'/0'/0/0",
      "curve_type": "secp256k1"
    }
  ]
}
```

**禁止字段**:
```json
{
  "mnemonic": "...",        // ❌ 禁止
  "private_key": "...",     // ❌ 禁止
  "wallet_password": "...", // ❌ 禁止
  "seed": "..."             // ❌ 禁止
}
```

**响应体**:
```json
{
  "success": true,
  "data": {
    "wallets": [
      {
        "id": "uuid-here",
        "chain": "ETH",
        "address": "0x742d...",
        "created_at": "2025-12-02T10:00:00Z"
      }
    ]
  }
}
```

---

## ⚠️ 用户教育文档

### 给用户的重要提示

```markdown
# 📢 重要声明：非托管钱包

## 您完全控制您的资产

✅ **好消息**:
- 您的私钥和助记词只存储在您的设备上
- 平台无法访问您的资产
- 平台被黑，您的资产仍然安全
- 您可以在任何钱包（MetaMask、Trust Wallet等）使用同一助记词

⚠️ **责任**:
- **必须妥善保管助记词**（这是恢复钱包的唯一方式）
- **助记词丢失 = 资产永久丢失**（无法找回）
- **不要截图或拍照**（防止云端泄露）
- **不要通过网络传输**（防止被拦截）
- **手写抄录到纸上**（最安全的方式）

## 如何备份助记词

1. ✍️ 准备纸和笔
2. 📝 抄写24个单词（按顺序）
3. ✅ 检查拼写和顺序
4. 🔒 存放到安全地方（保险柜/银行保管箱）
5. 🔄 制作多份备份（存放在不同地点）

## 不要相信任何要求助记词的人

⚠️ **诈骗警告**:
- 平台客服**永远不会**要求您提供助记词
- 任何要求助记词的行为都是**诈骗**
- 输入助记词前，**仔细检查网址**
- 只在官方网站输入助记词
```

---

## 🎯 实施检查清单

### 前端实施

- [x] 使用OS级随机数生成器（`crypto.getRandomValues`）
- [x] BIP39助记词生成（12或24个单词）
- [x] BIP32/BIP44密钥派生
- [x] 多链支持（EVM/BTC/SOL/TON）
- [x] AES-256-GCM加密
- [x] PBKDF2密钥派生（600,000迭代）
- [x] IndexedDB安全存储
- [x] 内存清零（使用zeroize）
- [ ] 助记词备份UI
- [ ] 用户教育文档
- [ ] 恢复钱包功能

### 后端实施

- [x] 删除 `encrypted_private_key` 字段
- [x] 删除托管化模块
- [x] API只接受公开信息
- [x] 验证地址格式
- [x] 防止重复地址
- [x] 审计日志
- [x] 数据库迁移
- [ ] API文档更新
- [ ] Swagger规范

### 安全审计

- [ ] 代码审查（内部）
- [ ] 渗透测试
- [ ] 第三方安全审计
- [ ] 开源代码接受社区审查
- [ ] Bug赏金计划

---

**文档版本**: 2.0  
**最后更新**: 2025-12-02  
**安全等级**: Production-Ready Non-Custodial

