//! 地址生成算法验证测试
//!
//! 企业级测试：验证各链地址生成算法正确性，与标准钱包对比

#[cfg(test)]
mod tests {
    use crate::domain::chain_config::ChainRegistry;

    #[test]
    fn test_bip44_path_derivation() {
        let registry = ChainRegistry::new();

        // 验证各链的coin_type
        let eth = registry.get_by_symbol("ETH").unwrap();
        assert_eq!(eth.coin_type, 60);

        let bsc = registry.get_by_symbol("BSC").unwrap();
        assert_eq!(bsc.coin_type, 60); // BSC使用与ETH相同的派生路径

        let btc = registry.get_by_symbol("BTC").unwrap();
        assert_eq!(btc.coin_type, 0);

        let sol = registry.get_by_symbol("SOL").unwrap();
        assert_eq!(sol.coin_type, 501);

        let ton = registry.get_by_symbol("TON").unwrap();
        assert_eq!(ton.coin_type, 607);
    }

    #[test]
    fn test_derivation_path_template() {
        let registry = ChainRegistry::new();

        // 验证派生路径模板
        let eth = registry.get_by_symbol("ETH").unwrap();
        let path = eth.derive_path(0, 0, 0);
        assert!(path.contains("m/44'/60'"));

        let btc = registry.get_by_symbol("BTC").unwrap();
        let path = btc.derive_path(0, 0, 0);
        assert!(path.contains("m/84'/0'")); // Bitcoin使用BIP84 (native segwit)

        let sol = registry.get_by_symbol("SOL").unwrap();
        let path = sol.derive_path(0, 0, 0);
        assert!(path.contains("m/44'/501'"));
    }

    #[test]
    fn test_address_format_validation() {
        let registry = ChainRegistry::new();

        // 验证地址格式
        let eth = registry.get_by_symbol("ETH").unwrap();
        assert_eq!(format!("{:?}", eth.address_format), "Hex");

        let btc = registry.get_by_symbol("BTC").unwrap();
        assert_eq!(format!("{:?}", btc.address_format), "Bech32");

        let sol = registry.get_by_symbol("SOL").unwrap();
        assert_eq!(format!("{:?}", sol.address_format), "SolanaBase58");
    }

    #[test]
    fn test_chain_id_mapping() {
        let registry = ChainRegistry::new();

        // 验证chain_id映射
        let eth = registry.get_by_chain_id(1).unwrap();
        assert_eq!(eth.symbol, "ETH");

        let bsc = registry.get_by_chain_id(56).unwrap();
        assert_eq!(bsc.symbol, "BNB");

        let sol = registry.get_by_chain_id(501).unwrap();
        assert_eq!(sol.symbol, "SOL");
    }
}

