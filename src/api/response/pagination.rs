// 统一分页响应格式

use serde::Serialize;

/// 统一分页响应结构
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub total_pages: u32,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, page: u32, page_size: u32, total: u64) -> Self {
        let total_pages = if page_size > 0 {
            ((total as f64) / (page_size as f64)).ceil() as u32
        } else {
            0
        };

        Self {
            data,
            page,
            page_size,
            total,
            total_pages,
        }
    }

    /// 从查询结果创建分页响应
    pub fn from_query(data: Vec<T>, page: u32, page_size: u32, total: u64) -> Self {
        Self::new(data, page, page_size, total)
    }
}

/// 分页参数
#[derive(Debug, Clone, Copy)]
pub struct PaginationParams {
    pub page: u32,
    pub page_size: u32,
}

impl PaginationParams {
    pub fn new(page: Option<u32>, page_size: Option<u32>) -> Self {
        Self {
            page: page.unwrap_or(1).max(1),
            page_size: page_size.unwrap_or(20).clamp(1, 100), // 限制在1-100之间
        }
    }

    pub fn offset(&self) -> i64 {
        ((self.page - 1) * self.page_size) as i64
    }

    pub fn limit(&self) -> i64 {
        self.page_size as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_params() {
        let params = PaginationParams::new(Some(2), Some(10));
        assert_eq!(params.page, 2);
        assert_eq!(params.page_size, 10);
        assert_eq!(params.offset(), 10);
        assert_eq!(params.limit(), 10);
    }

    #[test]
    fn test_paginated_response() {
        let data = vec![1, 2, 3];
        let response = PaginatedResponse::new(data, 1, 10, 25);
        assert_eq!(response.page, 1);
        assert_eq!(response.page_size, 10);
        assert_eq!(response.total, 25);
        assert_eq!(response.total_pages, 3);
    }
}
