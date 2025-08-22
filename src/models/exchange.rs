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
    pub totalEq: String,
    pub isoEq: String,
    pub adjEq: String,
    pub ordFroz: String,
    pub imr: String,
    pub mmr: String,
    pub cTime: String,
    pub uTime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkxBalanceDetail {
    pub ccy: String,
    pub eq: String,
    pub cashBal: String,
    pub uPL: String,
    pub equity: String,
    pub availBal: String,
    pub frozenBal: String,
    pub ordFrozen: String,
    pub liab: String,
    pub upl: String,
    pub uplLiab: String,
    pub crossLiab: String,
    pub isoLiab: String,
    pub mgnRatio: String,
    pub interest: String,
    pub notionalLever: String,
    pub adl: String,
    pub availPos: String,
    pub marginRatio: String,
    pub mgnMgnRatio: String,
    pub ordAvail: String,
    pub liqPx: String,
    pub uplPx: String,
    pub markPx: String,
    pub cTime: String,
    pub uTime: String,
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
