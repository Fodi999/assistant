use serde::{Deserialize, Serialize};

/// Pagination parameters extracted from query string
/// Usage: `Query<PaginationParams>` in handler
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationParams {
    /// Page number (1-indexed, default 1)
    pub page: Option<u32>,
    /// Items per page (default 50, max 100)
    pub per_page: Option<u32>,
}

impl PaginationParams {
    const DEFAULT_PAGE: u32 = 1;
    const DEFAULT_PER_PAGE: u32 = 50;
    const MAX_PER_PAGE: u32 = 100;

    pub fn page(&self) -> u32 {
        self.page.unwrap_or(Self::DEFAULT_PAGE).max(1)
    }

    pub fn per_page(&self) -> u32 {
        self.per_page
            .unwrap_or(Self::DEFAULT_PER_PAGE)
            .min(Self::MAX_PER_PAGE)
            .max(1)
    }

    /// SQL OFFSET value
    pub fn offset(&self) -> i64 {
        ((self.page() - 1) * self.per_page()) as i64
    }

    /// SQL LIMIT value
    pub fn limit(&self) -> i64 {
        self.per_page() as i64
    }
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total: i64, params: &PaginationParams) -> Self {
        let per_page = params.per_page();
        let total_pages = if total > 0 {
            ((total as u32) + per_page - 1) / per_page
        } else {
            0
        };

        Self {
            items,
            total,
            page: params.page(),
            per_page,
            total_pages,
        }
    }
}
