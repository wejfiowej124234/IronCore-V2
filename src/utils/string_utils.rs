//! 字符串工具模块
//! 提供字符串处理相关的工具函数

/// 截断字符串到指定长度
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// 移除字符串两端的空白字符
pub fn trim(s: &str) -> String {
    s.trim().to_string()
}

/// 检查字符串是否为空或只包含空白字符
pub fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

/// 安全地转换为字符串（处理None值）
pub fn to_string_opt<T: ToString>(opt: Option<T>) -> Option<String> {
    opt.map(|v| v.to_string())
}
