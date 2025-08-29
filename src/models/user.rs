use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub dingtalk_user_id: String,
    pub dingtalk_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserExchange {
    pub id: Uuid,
    pub user_id: Uuid,
    pub exchange_type: ExchangeType,
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: Option<String>, // 欧易需要
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum ExchangeType {
    Binance,
    Okx,
    Weex,
}

impl Default for ExchangeType {
    fn default() -> Self {
        ExchangeType::Binance
    }
}

impl std::fmt::Display for ExchangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExchangeType::Binance => write!(f, "币安"),
            ExchangeType::Okx => write!(f, "欧易"),
            ExchangeType::Weex => write!(f, "WEEX"),
        }
    }
}

impl std::str::FromStr for ExchangeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "binance" | "币安" => Ok(ExchangeType::Binance),
            "okx" | "欧易" => Ok(ExchangeType::Okx),
            "weex" => Ok(ExchangeType::Weex),
            _ => Err(format!("不支持的交易所类型: {}", s)),
        }
    }
}
