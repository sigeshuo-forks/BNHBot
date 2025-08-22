use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

// 币安API响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceAccountInfo {
    pub makerCommission: i64,
    pub takerCommission: i64,
    pub buyerCommission: i64,
    pub sellerCommission: i64,
    pub canTrade: bool,
    pub canWithdraw: bool,
    pub canDeposit: bool,
    pub updateTime: i64,
    pub accountType: String,
    pub balances: Vec<BinanceBalance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceBalance {
    pub asset: String,
    pub free: String,
    pub locked: String,
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

// WEEX API响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeexAccountInfo {
    pub code: i32,
    pub msg: String,
    pub data: WeexAccountData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeexAccountData {
    pub accountId: String,
    pub accountType: String,
    pub balances: Vec<WeexBalance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeexBalance {
    pub asset: String,
    pub free: String,
    pub locked: String,
    pub total: String,
}

// 通用余额结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeBalance {
    pub asset: String,
    pub free: Decimal,
    pub locked: Decimal,
    pub total: Decimal,
}
