// 国际化支持模块 - 基础实现

use std::collections::HashMap;
use std::sync::LazyLock;

/// 支持的语言
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Chinese,
    Japanese,
    Korean,
}

impl Language {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "zh" | "zh-cn" | "chinese" => Language::Chinese,
            "ja" | "japanese" => Language::Japanese,
            "ko" | "korean" => Language::Korean,
            _ => Language::English,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Chinese => "zh",
            Language::Japanese => "ja",
            Language::Korean => "ko",
        }
    }
}

/// 错误消息国际化
static ERROR_MESSAGES: LazyLock<HashMap<&'static str, HashMap<Language, &'static str>>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();

        // Wallet errors
        let mut wallet_not_found = HashMap::new();
        wallet_not_found.insert(Language::English, "Wallet not found");
        wallet_not_found.insert(Language::Chinese, "钱包不存在");
        wallet_not_found.insert(Language::Japanese, "ウォレットが見つかりません");
        wallet_not_found.insert(Language::Korean, "지갑을 찾을 수 없습니다");
        map.insert("wallet_not_found", wallet_not_found);

        // Balance errors
        let mut insufficient_balance = HashMap::new();
        insufficient_balance.insert(Language::English, "Insufficient balance");
        insufficient_balance.insert(Language::Chinese, "余额不足");
        insufficient_balance.insert(Language::Japanese, "残高不足");
        insufficient_balance.insert(Language::Korean, "잔액 부족");
        map.insert("insufficient_balance", insufficient_balance);

        // Transaction errors
        let mut transaction_failed = HashMap::new();
        transaction_failed.insert(Language::English, "Transaction failed");
        transaction_failed.insert(Language::Chinese, "交易失败");
        transaction_failed.insert(Language::Japanese, "取引に失敗しました");
        transaction_failed.insert(Language::Korean, "거래 실패");
        map.insert("transaction_failed", transaction_failed);

        // Network errors
        let mut network_error = HashMap::new();
        network_error.insert(Language::English, "Network error, please try again");
        network_error.insert(Language::Chinese, "网络错误，请重试");
        network_error.insert(Language::Japanese, "ネットワークエラー、再試行してください");
        network_error.insert(Language::Korean, "네트워크 오류, 다시 시도하세요");
        map.insert("network_error", network_error);

        map
    });

/// 获取本地化的错误消息
pub fn get_error_message(key: &str, lang: Language) -> String {
    ERROR_MESSAGES
        .get(key)
        .and_then(|msgs| msgs.get(&lang).copied())
        .unwrap_or_else(|| {
            // Fallback to English
            ERROR_MESSAGES
                .get(key)
                .and_then(|msgs| msgs.get(&Language::English).copied())
                .unwrap_or(key)
        })
        .to_string()
}

/// 格式化金额（支持国际化）
pub fn format_amount(amount: &str, lang: Language) -> String {
    // 基础实现，可以扩展为更复杂的格式化
    match lang {
        Language::Chinese => format!("{}", amount),
        Language::Japanese => format!("{}", amount),
        Language::Korean => format!("{}", amount),
        Language::English => format!("{}", amount),
    }
}

/// 格式化时间（支持国际化）
pub fn format_time(dt: &chrono::DateTime<chrono::Utc>, lang: Language) -> String {
    match lang {
        Language::Chinese => dt.format("%Y年%m月%d日 %H:%M:%S").to_string(),
        Language::Japanese => dt.format("%Y年%m月%d日 %H:%M:%S").to_string(),
        Language::Korean => dt.format("%Y년 %m월 %d일 %H:%M:%S").to_string(),
        Language::English => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_error_message() {
        assert_eq!(
            get_error_message("wallet_not_found", Language::English),
            "Wallet not found"
        );
        assert_eq!(
            get_error_message("wallet_not_found", Language::Chinese),
            "钱包不存在"
        );
    }

    #[test]
    fn test_language_from_str() {
        assert_eq!(Language::from_str("zh"), Language::Chinese);
        assert_eq!(Language::from_str("en"), Language::English);
        assert_eq!(Language::from_str("unknown"), Language::English);
    }
}
