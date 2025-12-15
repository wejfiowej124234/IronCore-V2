use std::collections::HashMap;

pub fn error_map() -> HashMap<&'static str, &'static str> {
    // 与前端 IronForge 强类型/文案对齐的基础错误映射
    HashMap::from([
        ("timeout", "请求超时，请稍后重试"),
        ("network", "网络异常，请检查连接"),
        ("rate_limit", "请求过于频繁，请稍后再试"),
        ("unauthorized", "认证失败或权限不足"),
        ("forbidden", "权限不足，拒绝访问"),
        ("bad_request", "请求参数错误"),
        ("not_found", "资源不存在"),
        ("internal", "服务开小差了，请稍后再试"),
    ])
}
