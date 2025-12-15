//! 费用体系非托管验证器
//!
//! P2级修复：确保费用体系完全非托管化
//! 所有费用必须由用户签名授权，后端不能代扣

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 费用类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FeeType {
    /// Gas费用（链上）
    GasFee,
    /// 服务费
    ServiceFee,
    /// 跨链桥费用
    BridgeFee,
    /// 交换费用
    SwapFee,
}

/// 费用计算结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCalculation {
    pub fee_type: FeeType,
    pub amount: String,
    pub currency: String,
    pub breakdown: Vec<FeeItem>,
    pub total: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeItem {
    pub name: String,
    pub amount: String,
    pub description: String,
}

/// 费用非托管验证器
pub struct FeeNonCustodialValidator;

impl FeeNonCustodialValidator {
    /// 验证费用计算是否符合非托管原则
    ///
    /// # 非托管原则
    /// 1. 所有费用必须明确告知用户
    /// 2. 费用必须包含在用户签名的交易中
    /// 3. 后端不能在用户不知情的情况下扣除费用
    /// 4. 费用必须是预先计算的，不能事后修改
    pub fn validate_fee_calculation(calculation: &FeeCalculation) -> Result<ValidationResult> {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();

        // 1. 验证金额格式
        if calculation.amount.is_empty() {
            issues.push("Fee amount is empty".to_string());
        }

        if calculation.total.is_empty() {
            issues.push("Total fee is empty".to_string());
        }

        // 2. 验证breakdown完整性
        if calculation.breakdown.is_empty() {
            warnings.push(
                "Fee breakdown is empty, user may not understand fee composition".to_string(),
            );
        }

        // 3. 验证费用类型
        match calculation.fee_type {
            FeeType::GasFee => {
                // Gas费用必须包含在签名交易中
                if !calculation
                    .breakdown
                    .iter()
                    .any(|item| item.name.contains("gas"))
                {
                    issues.push("Gas fee must be included in transaction".to_string());
                }
            }
            FeeType::ServiceFee => {
                // 服务费必须明确说明
                if calculation
                    .breakdown
                    .iter()
                    .any(|item| item.description.is_empty())
                {
                    warnings.push("Service fee should have clear description".to_string());
                }
            }
            _ => {}
        }

        // 4. 验证金额一致性
        let breakdown_total: f64 = calculation
            .breakdown
            .iter()
            .filter_map(|item| item.amount.parse::<f64>().ok())
            .sum();

        if let Ok(total) = calculation.total.parse::<f64>() {
            if (breakdown_total - total).abs() > 0.00001 {
                issues.push(format!(
                    "Fee breakdown total ({}) does not match total ({})",
                    breakdown_total, total
                ));
            }
        }

        Ok(ValidationResult {
            is_valid: issues.is_empty(),
            issues,
            warnings,
        })
    }

    /// 验证用户是否已授权费用支付
    ///
    /// # 检查项
    /// 1. 签名交易是否包含费用金额
    /// 2. 费用是否在用户授权范围内
    /// 3. 费用是否与预估一致
    pub fn validate_fee_authorization(
        _estimated_fee: &FeeCalculation,
        signed_tx: &str,
    ) -> Result<bool> {
        // 解析签名交易
        if !signed_tx.starts_with("0x") {
            anyhow::bail!("Invalid signed transaction format");
        }

        // TODO: 完整实现
        // 1. 解析RLP编码的交易
        // 2. 提取gasPrice和gasLimit
        // 3. 计算实际gas费用
        // 4. 验证与估算费用是否一致

        // 临时实现：基本验证
        Ok(true)
    }

    /// 生成费用明细（供用户签名前审查）
    pub fn generate_fee_disclosure(calculation: &FeeCalculation) -> FeeDisclosure {
        FeeDisclosure {
            fee_type: format!("{:?}", calculation.fee_type),
            total_amount: calculation.total.clone(),
            currency: calculation.currency.clone(),
            breakdown: calculation.breakdown.clone(),
            user_notice: vec![
                "This fee will be deducted from your wallet balance".to_string(),
                "Please review the fee breakdown carefully before signing".to_string(),
                "The transaction will not proceed without your signature".to_string(),
                "Backend cannot deduct fees without your authorization".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeDisclosure {
    pub fee_type: String,
    pub total_amount: String,
    pub currency: String,
    pub breakdown: Vec<FeeItem>,
    pub user_notice: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_fee_calculation() {
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
    fn test_validate_inconsistent_total() {
        let calculation = FeeCalculation {
            fee_type: FeeType::ServiceFee,
            amount: "10".to_string(),
            currency: "USD".to_string(),
            breakdown: vec![FeeItem {
                name: "service".to_string(),
                amount: "5".to_string(),
                description: "Service fee".to_string(),
            }],
            total: "10".to_string(), // 不匹配breakdown
        };

        let result = FeeNonCustodialValidator::validate_fee_calculation(&calculation).unwrap();
        assert!(!result.is_valid);
        assert!(!result.issues.is_empty());
    }
}
