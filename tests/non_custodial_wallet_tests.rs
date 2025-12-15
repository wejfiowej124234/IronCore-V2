//! 非托管钱包系统集成测试
//!
//! 测试覆盖：双锁机制、客户端签名、多链派生、跨链桥

#[cfg(test)]
mod tests {
    use ironcore::domain::derivation_path_validator::DerivationPathValidator;

    #[test]
    fn test_derivation_path_validation() {
        let validator = DerivationPathValidator::new();

        // 测试ETH标准路径
        assert!(validator.validate_path(1, "m/44'/60'/0'/0/0").is_ok());

        // 测试BSC路径（应与ETH相同）
        assert!(validator.validate_path(56, "m/44'/60'/0'/0/0").is_ok());

        // 测试BTC路径（BIP84）
        let btc_path = validator.build_path(0, 0, 0, 0).unwrap();
        assert!(btc_path.starts_with("m/84'/0'"));

        // 测试Solana路径
        assert!(validator.validate_path(501, "m/44'/501'/0'/0'").is_ok());
    }

    #[test]
    fn test_multi_chain_path_consistency() {
        let validator = DerivationPathValidator::new();

        let paths = vec![
            (1, "m/44'/60'/0'/0/0".to_string()),   // ETH
            (56, "m/44'/60'/0'/0/0".to_string()),  // BSC
            (137, "m/44'/60'/0'/0/0".to_string()), // Polygon
        ];

        // 所有EVM链应该使用相同的account和index
        assert!(validator.validate_multi_chain_consistency(&paths).is_ok());
    }

    #[test]
    fn test_inconsistent_paths_detection() {
        let validator = DerivationPathValidator::new();

        let paths = vec![
            (1, "m/44'/60'/0'/0/0".to_string()),
            (56, "m/44'/60'/1'/0/0".to_string()), // 不同的account
        ];

        // 应该检测到不一致
        assert!(validator.validate_multi_chain_consistency(&paths).is_err());
    }
}

#[cfg(test)]
mod fee_validator_tests {
    use ironcore::service::fee_non_custodial_validator::{
        FeeCalculation, FeeItem, FeeNonCustodialValidator, FeeType,
    };

    #[test]
    fn test_fee_calculation_validation() {
        let calculation = FeeCalculation {
            fee_type: FeeType::GasFee,
            amount: "0.001".to_string(),
            currency: "ETH".to_string(),
            breakdown: vec![
                FeeItem {
                    name: "base_gas".to_string(),
                    amount: "0.0008".to_string(),
                    description: "Base gas fee".to_string(),
                },
                FeeItem {
                    name: "priority_fee".to_string(),
                    amount: "0.0002".to_string(),
                    description: "Priority fee".to_string(),
                },
            ],
            total: "0.001".to_string(),
        };

        let result = FeeNonCustodialValidator::validate_fee_calculation(&calculation).unwrap();
        assert!(result.is_valid);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn test_fee_total_mismatch_detection() {
        let calculation = FeeCalculation {
            fee_type: FeeType::ServiceFee,
            amount: "10".to_string(),
            currency: "USD".to_string(),
            breakdown: vec![FeeItem {
                name: "service".to_string(),
                amount: "5".to_string(),
                description: "Service fee".to_string(),
            }],
            total: "10".to_string(), // 不匹配
        };

        let result = FeeNonCustodialValidator::validate_fee_calculation(&calculation).unwrap();
        assert!(!result.is_valid);
    }
}

#[cfg(test)]
mod log_sanitizer_tests {
    use ironcore::infrastructure::log_sanitizer_enhanced::LogSanitizer;
    use serde_json::json;

    #[test]
    fn test_sanitize_private_key() {
        let input = json!({
            "private_key": "0x1234567890abcdef1234567890abcdef",
            "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb6"
        });

        let sanitized = LogSanitizer::sanitize_json(&input);
        assert_eq!(sanitized["private_key"], "***REDACTED***");
    }

    #[test]
    fn test_sanitize_mnemonic() {
        let input = json!({
            "mnemonic": "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        });

        let sanitized = LogSanitizer::sanitize_json(&input);
        assert_eq!(sanitized["mnemonic"], "***REDACTED***");
    }

    #[test]
    fn test_sanitize_string_with_private_key() {
        let text = "Processing with key: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let sanitized = LogSanitizer::sanitize_string(text);

        assert!(sanitized.contains("***REDACTED***"));
    }
}

#[cfg(test)]
mod broadcast_reliability_tests {
    use ironcore::service::broadcast_reliability_enhancer::RetryStrategy;

    #[test]
    fn test_retry_strategy_default() {
        let strategy = RetryStrategy::default();
        assert_eq!(strategy.max_retries, 5);
        assert_eq!(strategy.initial_delay_ms, 1000);
        assert_eq!(strategy.backoff_multiplier, 2.0);
        assert_eq!(strategy.max_delay_ms, 30000);
    }
}
