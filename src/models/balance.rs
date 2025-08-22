use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Balance {
    pub id: Uuid,
    pub user_id: Uuid,
    pub exchange_type: crate::models::ExchangeType,
    pub asset: String,
    pub free: Decimal,
    pub locked: Decimal,
    pub total: Decimal,
    pub usdt_value: Option<Decimal>,
    pub recorded_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSummary {
    pub user_id: Uuid,
    pub user_name: String,
    pub exchange_type: crate::models::ExchangeType,
    pub total_usdt_value: Decimal,
    pub balances: Vec<Balance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReport {
    pub date: String,
    pub total_users: usize,
    pub total_value: Decimal,
    pub user_summaries: Vec<BalanceSummary>,
}
