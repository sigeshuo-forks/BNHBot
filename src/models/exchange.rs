use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

// 币安现货API响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceSpotAccountInfo {
    pub makerCommission: i64,
    pub takerCommission: i64,
    pub buyerCommission: i64,
    pub sellerCommission: i64,
    pub canTrade: bool,
    pub canWithdraw: bool,
    pub canDeposit: bool,
    pub updateTime: i64,
    pub accountType: String,
    pub balances: Vec<BinanceSpotBalance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceSpotBalance {
    pub asset: String,
    pub free: String,
    pub locked: String,
}

// 币安合约账户API响应结构 (USDT-M) - 简化版本，只保留必要字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceFuturesAccountInfo {
    // 只保留我们需要的字段
    pub assets: Vec<BinanceFuturesAsset>,
    
    // 使用flatten来忽略所有其他字段，避免解析错误
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceFuturesAsset {
    pub asset: String,
    pub walletBalance: String,
    pub availableBalance: String,
    
    // 可选字段，如果不存在则为默认值
    #[serde(default)]
    pub positionInitialMargin: Option<String>,
    
    // 使用flatten来忽略所有其他字段
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

// 欧易API响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkxAccountBalance {
    pub code: String,
    pub msg: String,
    pub data: Vec<OkxBalanceData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkxBalanceData {
    pub details: Vec<OkxBalanceDetail>,
    
    // 总权益（USDT计价）
    #[serde(default)]
    pub totalEq: Option<String>,
    
    // 使用flatten来捕获所有其他字段，避免解析错误
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkxBalanceDetail {
    // 必需字段
    pub ccy: String,
    pub eq: String,
    pub availBal: String,
    pub frozenBal: String,
    
    // 使用flatten来捕获所有其他字段，避免解析错误
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

// WEEX API响应结构 (V2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeexAccountInfo {
    pub code: String,
    pub msg: String,
    #[serde(rename = "requestTime")]
    pub request_time: i64,
    pub data: Vec<WeexBalance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeexBalance {
    #[serde(rename = "coinId")]
    pub coin_id: i32,
    #[serde(rename = "coinName")]
    pub coin_name: String,
    pub available: String,
    pub frozen: String,
    pub equity: String,
}

// 通用余额结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeBalance {
    pub asset: String,
    pub free: Decimal,
    pub locked: Decimal,
    pub total: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usdt_value: Option<Decimal>, // 单个币种的USDT估值
}

// 账户总览结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSummary {
    pub total_usdt_value: Decimal, // 账户总USDT估值
    pub balances: Vec<ExchangeBalance>,
}
