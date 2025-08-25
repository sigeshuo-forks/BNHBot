use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

/// 余额历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceHistory {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub exchange_type: String,
    pub total_usdt_value: Decimal,
    pub balance_details: String, // JSON格式的详细余额信息
    pub recorded_date: String,   // YYYY-MM-DD格式
    pub recorded_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// 排名条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingEntry {
    pub user_id: Uuid,
    pub user_name: String,
    pub exchange_type: String,
    pub current_balance: Decimal,
    pub previous_balance: Option<Decimal>,
    pub change_amount: Decimal,
    pub change_percentage: Decimal,
    pub rank: u32,
    pub is_doubled: bool, // 是否实现翻倍
    pub balance_history: Vec<BalanceHistoryPoint>, // 用于绘制图表
}

/// 余额历史点（用于图表）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceHistoryPoint {
    pub date: String,
    pub balance: Decimal,
}

/// 排名响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingResponse {
    pub daily_rankings: Vec<RankingEntry>,
    pub weekly_rankings: Vec<RankingEntry>,
    pub monthly_rankings: Vec<RankingEntry>,
    pub last_updated: DateTime<Utc>,
}

/// 排名请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingRequest {
    pub period: RankingPeriod,
    pub limit: Option<u32>,
}

/// 排名周期
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RankingPeriod {
    Daily,
    Weekly,
    Monthly,
}

/// 钉钉排名消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkRankingMessage {
    pub top_rankings: Vec<RankingEntry>,
    pub total_participants: u32,
    pub ranking_url: String,
}

/// 余额收集请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceCollectionRequest {
    pub user_id: Uuid,
    pub user_name: String,
    pub exchange_type: String,
}

/// 余额收集响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceCollectionResponse {
    pub success: bool,
    pub message: String,
    pub collected_count: u32,
    pub failed_count: u32,
    pub errors: Vec<String>,
}
