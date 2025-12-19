# 非托管钱包双锁体系（企业级标准）

**更新日期**: 2025-12-02  
**安全等级**: 🔴 Production-Grade  
**架构类型**: Pure Non-Custodial Dual-Lock System

---

## 🔐 非托管双锁的正确定义

### ❌ 错误的"双锁"（托管模式）

```
托管模式双锁（已删除）:
锁1: 服务端主密钥  ❌ 后端持有
锁2: 用户密码      ❌ 上传给后端

问题: 后端可以同时获得两把钥匙 → 能解密私钥 → 托管化
```

### ✅ 正确的"双锁"（非托管模式）

```
非托管模式双锁:
锁1 (账户锁): 登录密码
  - 用途: 登录后端账户，管理用户profile
  - 后端存储: Argon2id哈希
  - 不涉及: 链上私钥、助记词

锁2 (钱包锁): 钱包密码
  - 用途: 本地解锁钱包，签名交易
  - 前端存储: 不存储（仅派生加密密钥）
  - 用于加密: 助记词（本地IndexedDB）
  
关键: 两把锁完全独立，后端只知道锁1
```

---

## 🎯 双锁体系完整流程

### 场景1: 用户注册

```typescript
// Step 1: 注册后端账户（锁1）
async function registerAccount(email: string, accountPassword: string) {
  // 1.1 后端验证密码强度
  // 1.2 后端使用Argon2id哈希密码
  // 1.3 存储到数据库
  
  const response = await fetch("/api/v1/auth/register", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      email,
      password: accountPassword // ✅ 账户密码（登录用）
    })
  });
  
  if (response.ok) {
    const result = await response.json();
    if (result.code === 0 && result.data?.access_token) {
      localStorage.setItem("auth_token", result.data.access_token);
    }
  }
}

// Step 2: 创建钱包（锁2）
async function createWallet(walletPassword: string) {
  // 2.1 前端生成助记词
  const mnemonic = generateMnemonic(24);
  
  // 2.2 前端使用钱包密码加密助记词
  const encrypted = await encryptMnemonic(mnemonic, walletPassword);
  
  // 2.3 存储到本地IndexedDB
  await saveToIndexedDB({
    encryptedMnemonic: encrypted,
    // ❌ 不存储: walletPassword（只用于派生密钥）
  });
  
  // 2.4 派生地址，发送到后端（仅公开信息）
  const addresses = deriveAddresses(mnemonic);
  await registerAddressesWithBackend(addresses);
  
  // 2.5 清除助记词
  mnemonic.fill(0);
}
```

---

### 场景2: 用户登录

```typescript
// Step 1: 使用账户密码登录后端（锁1）
async function loginAccount(email: string, accountPassword: string) {
  const response = await fetch("/api/v1/auth/login", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      email,
      password: accountPassword // ✅ 账户密码
    })
  });
  
  if (response.ok) {
    const result = await response.json();
    if (result.code === 0 && result.data?.access_token) {
      localStorage.setItem("auth_token", result.data.access_token);
    }
    if (result.code === 0 && result.data?.user) {
      localStorage.setItem("user", JSON.stringify(result.data.user));
    }
    
    // ✅ 登录成功，但钱包仍然锁定
    console.log("✅ 账户已登录");
    console.log("🔒 钱包仍然锁定（需要钱包密码）");
  }
}

// Step 2: 用户需要签名交易时，解锁钱包（锁2）
async function unlockWalletForTransaction(walletPassword: string) {
  // 2.1 从IndexedDB加载加密的助记词
  const { encryptedMnemonic } = await loadFromIndexedDB();
  
  // 2.2 使用钱包密码解密助记词
  try {
    const mnemonic = await decryptMnemonic(encryptedMnemonic, walletPassword);
    
    // 2.3 派生私钥（临时内存）
    const privateKey = derivePrivateKey(mnemonic, "m/44'/60'/0'/0/0");
    
    // 2.4 签名交易
    const signedTx = signTransaction(privateKey, transactionParams);
    
    // 2.5 立即清零私钥和助记词
    privateKey.fill(0);
    mnemonic.fill(0);
    
    // 2.6 发送已签名交易到后端
    await broadcastTransaction(signedTx);
    
    console.log("✅ 钱包已解锁并签名交易");
  } catch (error) {
    console.error("❌ 钱包密码错误");
  }
}
```

---

### 场景3: 跨链桥操作

```typescript
// 用户需要使用跨链桥时
async function executeBridgeTransfer(params: BridgeParams) {
  // 1. 检查是否登录（锁1）
  const jwt_token = localStorage.getItem("auth_token");
  if (!jwt_token) {
    throw new Error("请先登录账户");
  }
  
  // 2. 弹出钱包密码输入框（锁2）
  const walletPassword = await promptWalletPassword();
  
  // 3. 解锁钱包并签名源链交易
  const { encryptedMnemonic } = await loadFromIndexedDB();
  const mnemonic = await decryptMnemonic(encryptedMnemonic, walletPassword);
  const privateKey = derivePrivateKey(mnemonic, params.sourcePath);
  
  // 4. 签名源链交易
  const signedTx = signBridgeTransaction(privateKey, {
    from: params.sourceAddress,
    to: BRIDGE_CONTRACT_ADDRESS,
    value: params.amount,
    data: encodeBridgeData(params)
  });
  
  // 5. 清零敏感数据
  privateKey.fill(0);
  mnemonic.fill(0);
  
  // 6. 发送到后端（只发送已签名交易）
  const response = await fetch("/api/v1/bridge/execute", {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${jwt_token}`,
      "Content-Type": "application/json"
    },
    body: JSON.stringify({
      signed_source_tx: signedTx, // ✅ 已签名交易
      source_chain: params.sourceChain,
      destination_chain: params.destinationChain,
      // ❌ 不发送: walletPassword, privateKey, mnemonic
    })
  });
  
  console.log("✅ 跨链交易已签名并发送");
}
```

---

## 🔒 双锁体系对比表

| 特性 | 锁1（账户锁） | 锁2（钱包锁） |
|-----|-------------|-------------|
| **名称** | 账户密码 / 登录密码 | 钱包密码 / 解锁密码 |
| **用途** | 登录后端账户 | 解锁本地钱包、签名交易 |
| **涉及资产** | ❌ 不涉及链上资产 | ✅ 控制链上资产 |
| **后端知道** | ✅ 知道（哈希存储） | ❌ 不知道 |
| **存储位置** | 后端数据库（Argon2 hash） | 不存储（仅派生密钥） |
| **可重置** | ✅ 可重置（邮箱验证） | ❌ 不可重置（丢失=永久丢失） |
| **强度要求** | 8位+大小写+数字 | 12位+大小写+数字+特殊字符 |
| **过期时间** | JWT: 7天 | 会话: 15分钟（自动锁定） |
| **输入频率** | 每次登录 | 每次签名交易 |

---

## 📊 密码管理流程图

```
┌─────────────────────────────────────────────────────────┐
│                  用户设置密码                            │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  注册时:                                                │
│    ├─ 设置账户密码（锁1）                               │
│    │  └─ 输入: alice@example.com / MyAccount123        │
│    │     └─ 后端存储: Argon2id(MyAccount123)          │
│    │                                                    │
│    └─ 设置钱包密码（锁2）                               │
│       └─ 输入: MySecureWallet@2025                     │
│          └─ 前端: PBKDF2(MySecureWallet@2025, 600k)   │
│             └─ 用于加密助记词                          │
│                                                         │
└─────────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────────┐
│                  日常使用流程                            │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  每次登录:                                              │
│    1. 输入账户密码（锁1）                               │
│       ├─ POST /api/v1/auth/login                       │
│       └─ 返回JWT token                                 │
│                                                         │
│  查看余额:                                              │
│    ✅ 不需要钱包密码                                    │
│    ├─ 使用JWT token查询                                │
│    └─ GET /api/v1/balance                              │
│                                                         │
│  发送交易:                                              │
│    ⚠️ 需要钱包密码（锁2）                               │
│    1. 弹出钱包密码输入框                                │
│    2. 解密助记词                                        │
│    3. 派生私钥                                          │
│    4. 签名交易                                          │
│    5. 清零私钥                                          │
│    6. 发送已签名交易                                    │
│                                                         │
│  15分钟无操作:                                          │
│    🔒 钱包自动锁定                                      │
│    ✅ 账户仍然登录（JWT有效）                           │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

## 🎯 用户体验优化

### 会话管理

```typescript
class WalletSessionManager {
  private sessionTimeout = 15 * 60 * 1000; // 15分钟
  private sessionTimer: NodeJS.Timeout | null = null;
  
  // 解锁钱包
  async unlockWallet(walletPassword: string): Promise<void> {
    // 1. 验证密码
    const mnemonic = await this.decryptMnemonic(walletPassword);
    
    // 2. 派生并缓存主密钥（不缓存助记词）
    const masterKey = deriveMasterKey(mnemonic);
    mnemonic.fill(0);
    
    // 3. 存储到内存（加密）
    this.cachedMasterKey = masterKey;
    this.isUnlocked = true;
    
    // 4. 启动自动锁定计时器
    this.resetSessionTimer();
    
    console.log("✅ 钱包已解锁（15分钟内有效）");
  }
  
  // 重置计时器（每次使用钱包时调用）
  private resetSessionTimer(): void {
    if (this.sessionTimer) {
      clearTimeout(this.sessionTimer);
    }
    
    this.sessionTimer = setTimeout(() => {
      this.lockWallet();
    }, this.sessionTimeout);
  }
  
  // 锁定钱包
  private lockWallet(): void {
    // 清零缓存的主密钥
    if (this.cachedMasterKey) {
      this.cachedMasterKey.fill(0);
      this.cachedMasterKey = null;
    }
    
    this.isUnlocked = false;
    console.log("🔒 钱包已自动锁定（超时）");
    
    // 通知UI
    this.notifyLocked();
  }
  
  // 签名交易（自动重置计时器）
  async signTransaction(tx: Transaction): Promise<string> {
    if (!this.isUnlocked) {
      throw new Error("钱包已锁定，请先解锁");
    }
    
    // 1. 使用缓存的主密钥派生私钥
    const privateKey = derivePrivateKey(this.cachedMasterKey, tx.path);
    
    // 2. 签名
    const signedTx = signTransaction(privateKey, tx);
    
    // 3. 清零私钥
    privateKey.fill(0);
    
    // 4. 重置计时器
    this.resetSessionTimer();
    
    return signedTx;
  }
}
```

---

## ⚠️ 密码重置策略

### 账户密码（锁1）- 可重置

```
流程:
1. 用户点击"忘记账户密码"
2. 输入注册邮箱
3. 后端发送验证邮件
4. 用户点击邮件中的链接
5. 设置新的账户密码
6. ✅ 重置成功，使用新密码登录

影响:
✅ 可以重新登录账户
✅ 可以查看钱包列表和余额
⚠️ 如果忘记钱包密码，仍然无法签名交易
```

### 钱包密码（锁2）- 不可重置

```
场景: 用户忘记钱包密码

后果:
❌ 无法解密助记词
❌ 无法签名交易
❌ 无法使用钱包

解决方案:
1. 如果之前备份了助记词:
   ✅ 使用"恢复钱包"功能
   ✅ 输入备份的助记词
   ✅ 设置新的钱包密码
   ✅ 重新加密存储

2. 如果没有备份助记词:
   ❌ 资产永久丢失
   ❌ 无法找回
   ⚠️ 这就是非托管钱包的代价
```

---

## 📝 安全建议

### 给开发者

1. **永远不要上传钱包密码到后端**
   ```typescript
   // ❌ 错误
   await fetch("/api/v1/wallets/unlock", {
     body: JSON.stringify({ wallet_password })
   });
   
   // ✅ 正确
   const mnemonic = await decryptMnemonicLocally(wallet_password);
   ```

2. **使用强密钥派生函数**
   ```typescript
   // ✅ 正确: 600,000迭代
   PBKDF2(password, salt, 600_000, "SHA-256")
   
   // ❌ 错误: 迭代次数不足
   PBKDF2(password, salt, 1000, "SHA-256")
   ```

3. **立即清零敏感数据**
   ```typescript
   // ✅ 正确
   const privateKey = derivePrivateKey(mnemonic);
   const signedTx = sign(privateKey);
   privateKey.fill(0); // 立即清零
   
   // ❌ 错误: 私钥留在内存中
   const privateKey = derivePrivateKey(mnemonic);
   return sign(privateKey);
   ```

### 给用户

1. **设置不同的密码**
   - 账户密码: MyAccount2025@
   - 钱包密码: MyWallet!Secure#2025
   - ⚠️ 不要使用相同密码

2. **记住钱包密码**
   - 写在纸上（和助记词一起）
   - 或使用密码管理器
   - ⚠️ 丢失=无法使用钱包

3. **理解两把锁的区别**
   - 忘记账户密码 → 可以重置
   - 忘记钱包密码 → 需要助记词恢复

---

## 🎯 实施检查清单

### 前端

- [x] 区分账户密码和钱包密码
- [x] 钱包密码不上传到后端
- [x] 钱包密码不存储（仅派生密钥）
- [x] 15分钟会话超时
- [x] 自动锁定机制
- [ ] 钱包锁定状态UI
- [ ] 解锁钱包弹窗
- [ ] 密码强度提示

### 后端

- [x] 账户密码使用Argon2id哈希
- [x] 不接受钱包密码参数
- [x] 不存储钱包密码
- [x] JWT过期时间（7天）
- [x] 密码重置邮件
- [ ] API文档更新

### 用户教育

- [ ] 双锁概念说明
- [ ] 密码设置指南
- [ ] 密码重置流程说明
- [ ] 常见问题FAQ

---

**文档版本**: 2.0  
**最后更新**: 2025-12-02  
**架构类型**: Pure Non-Custodial Dual-Lock System

