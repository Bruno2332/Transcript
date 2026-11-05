use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PageResponse<T> {
    pub items: Vec<T>,
    pub page: i64,
    pub size: i64,
    pub total: i64,
}