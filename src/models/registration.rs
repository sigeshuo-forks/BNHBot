use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 报名状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegistrationStatus {
    Pending,    // 待审核
    Approved,   // 已通过
    Rejected,   // 已拒绝
}

impl Default for RegistrationStatus {
    fn default() -> Self {
        RegistrationStatus::Pending
    }
}

impl std::fmt::Display for RegistrationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistrationStatus::Pending => write!(f, "待审核"),
            RegistrationStatus::Approved => write!(f, "已通过"),
            RegistrationStatus::Rejected => write!(f, "已拒绝"),
        }
    }
}

/// 交易所类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegistrationExchangeType {
    Binance,    // 币安
    OKX,        // 欧易
    WEEX,       // WEEX
}

impl std::fmt::Display for RegistrationExchangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistrationExchangeType::Binance => write!(f, "币安"),
            RegistrationExchangeType::OKX => write!(f, "欧易"),
            RegistrationExchangeType::WEEX => write!(f, "WEEX"),
        }
    }
}

/// 报名请求
#[derive(Debug, Deserialize)]
pub struct RegistrationRequest {
    pub user_name: String,
    pub exchange: String,
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: Option<String>,
}

/// 报名记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registration {
    pub id: Uuid,
    pub user_name: String,
    pub exchange: RegistrationExchangeType,
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: Option<String>,
    pub status: RegistrationStatus,
    pub admin_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

/// 报名响应
#[derive(Debug, Serialize)]
pub struct RegistrationResponse {
    pub success: bool,
    pub message: String,
    pub registration_id: Option<Uuid>,
}

/// 报名审核请求
#[derive(Debug, Deserialize)]
pub struct RegistrationReview {
    pub registration_id: Uuid,
    pub status: RegistrationStatus,
    pub admin_notes: Option<String>,
}

/// 报名统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationStats {
    pub total: usize,
    pub pending: usize,
    pub approved: usize,
    pub rejected: usize,
    pub by_exchange: std::collections::HashMap<String, usize>,
}
