// RPC响应校验模块 - 防止链上数据污染

use anyhow::{Context, Result};

/// 验证RPC返回的余额值
pub fn validate_balance(balance_hex: &str) -> Result<u128> {
    // 移除0x前缀
    let balance_hex = balance_hex.trim_start_matches("0x");

    // 验证长度（最多32字节 = 64个十六进制字符）
    if balance_hex.len() > 64 {
        anyhow::bail!("Balance hex string too long: {}", balance_hex.len());
    }

    // 解析为u128
    let balance =
        u128::from_str_radix(balance_hex, 16).context("Failed to parse balance from hex")?;

    // 验证余额范围（防止溢出或异常值）
    // 最大余额：2^128 - 1，但实际代币总量通常远小于此
    // 这里设置一个合理的上限：10^30（约等于1万亿个以太币）
    const MAX_REASONABLE_BALANCE: u128 = 1_000_000_000_000_000_000_000_000_000_000_000;
    if balance > MAX_REASONABLE_BALANCE {
        anyhow::bail!("Balance exceeds reasonable maximum: {}", balance);
    }

    Ok(balance)
}

/// 验证RPC返回的nonce值
pub fn validate_nonce(nonce_hex: &str) -> Result<u64> {
    // 移除0x前缀
    let nonce_hex = nonce_hex.trim_start_matches("0x");

    // 验证长度（u64最多16个十六进制字符）
    if nonce_hex.len() > 16 {
        anyhow::bail!("Nonce hex string too long: {}", nonce_hex.len());
    }

    // 解析为u64
    let nonce = u64::from_str_radix(nonce_hex, 16).context("Failed to parse nonce from hex")?;

    Ok(nonce)
}

/// 验证RPC返回的gas值
pub fn validate_gas(gas_hex: &str) -> Result<u64> {
    // 移除0x前缀
    let gas_hex = gas_hex.trim_start_matches("0x");

    // 验证长度
    if gas_hex.len() > 16 {
        anyhow::bail!("Gas hex string too long: {}", gas_hex.len());
    }

    // 解析为u64
    let gas = u64::from_str_radix(gas_hex, 16).context("Failed to parse gas from hex")?;

    // 验证gas范围（防止异常值）
    // 最大gas limit：通常不超过30,000,000
    const MAX_REASONABLE_GAS: u64 = 30_000_000;
    if gas > MAX_REASONABLE_GAS {
        anyhow::bail!("Gas exceeds reasonable maximum: {}", gas);
    }

    Ok(gas)
}

/// 验证交易哈希格式
pub fn validate_tx_hash(tx_hash: &str) -> Result<String> {
    // 移除0x前缀
    let hash = tx_hash.trim_start_matches("0x");

    // 验证长度（以太坊交易哈希为32字节 = 64个十六进制字符）
    if hash.len() != 64 {
        anyhow::bail!(
            "Invalid transaction hash length: expected 64, got {}",
            hash.len()
        );
    }

    // 验证是否为有效的十六进制字符串
    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        anyhow::bail!("Invalid transaction hash format: contains non-hex characters");
    }

    Ok(format!("0x{}", hash))
}

/// 验证地址格式
pub fn validate_address(address: &str) -> Result<String> {
    // 移除0x前缀
    let addr = address.trim_start_matches("0x");

    // 验证长度（以太坊地址为20字节 = 40个十六进制字符）
    if addr.len() != 40 {
        anyhow::bail!("Invalid address length: expected 40, got {}", addr.len());
    }

    // 验证是否为有效的十六进制字符串
    if !addr.chars().all(|c| c.is_ascii_hexdigit()) {
        anyhow::bail!("Invalid address format: contains non-hex characters");
    }

    // 转换为小写（以太坊地址不区分大小写）
    Ok(format!("0x{}", addr.to_lowercase()))
}

/// 验证RPC响应格式
pub fn validate_rpc_response(json: &serde_json::Value) -> Result<()> {
    // 检查是否有error字段
    if let Some(error) = json.get("error") {
        let error_code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
        let error_msg = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        anyhow::bail!("RPC error {}: {}", error_code, error_msg);
    }

    // 检查是否有result字段
    if json.get("result").is_none() {
        anyhow::bail!("Missing result field in RPC response");
    }

    // 检查jsonrpc版本
    if let Some(version) = json.get("jsonrpc") {
        if version.as_str() != Some("2.0") {
            anyhow::bail!("Unsupported JSON-RPC version: {:?}", version);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_balance() {
        assert!(validate_balance("0x1bc16d674ec80000").is_ok()); // 2 ETH
        assert!(validate_balance("0x0").is_ok()); // 0
        assert!(validate_balance("invalid").is_err());
    }

    #[test]
    fn test_validate_nonce() {
        assert!(validate_nonce("0x5").is_ok());
        assert!(validate_nonce("0x0").is_ok());
        assert!(validate_nonce("invalid").is_err());
    }

    #[test]
    fn test_validate_tx_hash() {
        let hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        assert!(validate_tx_hash(hash).is_ok());
        assert!(validate_tx_hash("invalid").is_err());
    }

    #[test]
    fn test_validate_address() {
        // 使用42个字符的完整地址（20字节 = 40个hex + 0x前缀）
        let addr = "0x742d35cc6634c0532925a3b844bc9e7595f0beb6";
        assert!(validate_address(addr).is_ok());
        assert!(validate_address("invalid").is_err());
        assert!(validate_address("0x123").is_err()); // 太短
    }
}
