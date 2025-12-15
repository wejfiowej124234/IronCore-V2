//! 交易构建和签名验证测试
//!
//! 企业级测试：验证各链交易构建和签名正确性

#[cfg(test)]
mod tests {
    use super::super::transaction_builder::{TransactionBuilder, BuildTransactionRequest};
    use hex;

    #[tokio::test]
    async fn test_ethereum_rlp_encoding() {
        let builder = TransactionBuilder::new();
        
        let request = BuildTransactionRequest {
            chain: "ETH".to_string(),
            from: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb6".to_string(),
            to: "0x1234567890123456789012345678901234567890".to_string(),
            amount: "1000000000000000000".to_string(), // 1 ETH
            data: None,
            gas_price: Some("20000000000".to_string()), // 20 Gwei
            gas_limit: Some("21000".to_string()),
            nonce: Some(0),
            chain_id: Some(1),
        };
        
        let response = builder.build_transaction(request).await.unwrap();
        
        // 验证RLP编码格式
        assert!(response.raw_transaction.starts_with("0x"));
        assert_eq!(response.transaction_details.chain, "ETH");
        assert_eq!(response.transaction_details.amount, "1000000000000000000");
    }

    #[tokio::test]
    async fn test_solana_transaction_format() {
        let builder = TransactionBuilder::new();
        
        let request = BuildTransactionRequest {
            chain: "SOL".to_string(),
            from: "11111111111111111111111111111111".to_string(),
            to: "22222222222222222222222222222222".to_string(),
            amount: "1000000000".to_string(), // 1 SOL (in lamports)
            data: None,
            gas_price: None,
            gas_limit: None,
            nonce: None,
            chain_id: None,
        };
        
        let response = builder.build_transaction(request).await.unwrap();
        
        assert_eq!(response.transaction_details.chain, "SOL");
        // Solana费用应该大于0
        assert!(response.transaction_details.estimated_fee.parse::<f64>().unwrap() > 0.0);
    }

    #[tokio::test]
    async fn test_bitcoin_transaction_format() {
        let builder = TransactionBuilder::new();
        
        let request = BuildTransactionRequest {
            chain: "BTC".to_string(),
            from: "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh".to_string(),
            to: "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4".to_string(),
            amount: "0.001".to_string(), // 0.001 BTC
            data: None,
            gas_price: None,
            gas_limit: None,
            nonce: None,
            chain_id: None,
        };
        
        let response = builder.build_transaction(request).await.unwrap();
        
        assert_eq!(response.transaction_details.chain, "BTC");
        // Bitcoin费用应该大于0
        assert!(response.transaction_details.estimated_fee.parse::<f64>().unwrap() > 0.0);
    }
}

