use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// 报名状态
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum RegistrationStatus {
    Pending,    // 待审核
    Approved,   // 已通过
    Rejected,   // 已拒绝
    Cancelled,  // 已取消
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
            RegistrationStatus::Cancelled => write!(f, "已取消"),
        }
    }
}

/// 报名类型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum RegistrationType {
    Exchange,   // 交易所API绑定
    Event,      // 活动报名
    Training,   // 培训报名
    Other,      // 其他
}

impl Default for RegistrationType {
    fn default() -> Self {
        RegistrationType::Exchange
    }
}

impl std::fmt::Display for RegistrationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistrationType::Exchange => write!(f, "交易所API绑定"),
            RegistrationType::Event => write!(f, "活动报名"),
            RegistrationType::Training => write!(f, "培训报名"),
            RegistrationType::Other => write!(f, "其他"),
        }
    }
}

/// 报名记录
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Registration {
    pub id: Uuid,
    pub user_id: Uuid,
    pub registration_type: RegistrationType,
    pub title: String,
    pub content: String,
    pub status: RegistrationStatus,
    pub admin_notes: Option<String>,
    pub admin_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

/// 报名表单数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationForm {
    pub registration_type: RegistrationType,
    pub title: String,
    pub content: String,
    pub contact_info: String,
    pub additional_fields: Option<serde_json::Value>,
}

/// 报名审核请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationReview {
    pub registration_id: Uuid,
    pub status: RegistrationStatus,
    pub admin_notes: Option<String>,
    pub admin_id: Uuid,
}

/// 报名统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationStats {
    pub total: usize,
    pub pending: usize,
    pub approved: usize,
    pub rejected: usize,
    pub cancelled: usize,
    pub by_type: std::collections::HashMap<String, usize>,
}
