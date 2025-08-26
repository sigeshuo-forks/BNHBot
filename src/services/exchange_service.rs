use crate::models::{ExchangeType, ExchangeBalance, UserExchange};
use crate::models::exchange::*;
use anyhow::Result;
use reqwest::Client;
use rust_decimal::Decimal;
use std::time::Duration;
use base64::Engine;

#[derive(Clone)]
pub struct ExchangeService {
    client: Client,
}

impl ExchangeService {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self { client }
    }

    pub async fn get_account_balance(&self, user_exchange: &UserExchange) -> Result<Vec<ExchangeBalance>> {
        match user_exchange.exchange_type {
            ExchangeType::Binance => self.get_binance_balance(user_exchange).await,
            ExchangeType::Okx => self.get_okx_balance(user_exchange).await,
            ExchangeType::Weex => self.get_weex_balance(user_exchange).await,
        }
    }

    pub async fn get_account_summary(&self, user_exchange: &UserExchange) -> Result<AccountSummary> {
        match user_exchange.exchange_type {
            ExchangeType::Binance => self.get_binance_summary(user_exchange).await,
            ExchangeType::Okx => self.get_okx_summary(user_exchange).await,
            ExchangeType::Weex => self.get_weex_summary(user_exchange).await,
        }
    }

    async fn get_binance_balance(&self, user_exchange: &UserExchange) -> Result<Vec<ExchangeBalance>> {
        let url = "https://api.binance.com/api/v3/account";
        let timestamp = chrono::Utc::now().timestamp_millis();
        
        let signature = self.generate_binance_signature(
            &format!("timestamp={}", timestamp),
            &user_exchange.secret_key,
        );

        let response = self.client
            .get(url)
            .header("X-MBX-APIKEY", &user_exchange.api_key)
            .query(&[("timestamp", timestamp.to_string()), ("signature", signature)])
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("币安API调用失败: {}", response.status());
        }

        let account_info: BinanceAccountInfo = response.json().await?;
        
        let balances: Vec<ExchangeBalance> = account_info.balances
            .into_iter()
            .filter_map(|b| {
                let free: Decimal = b.free.parse().ok()?;
                let locked: Decimal = b.locked.parse().ok()?;
                
                if free > Decimal::ZERO || locked > Decimal::ZERO {
                    Some(ExchangeBalance {
                        asset: b.asset,
                        free,
                        locked,
                        total: free + locked,
                        usdt_value: None,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(balances)
    }

    async fn get_binance_summary(&self, user_exchange: &UserExchange) -> Result<AccountSummary> {
        let balances = self.get_binance_balance(user_exchange).await?;
        
        // 只统计USDT余额，忽略其他币种
        let mut total_usdt_value = Decimal::ZERO;
        let mut usdt_balances = Vec::new();
        
        for balance in balances {
            if balance.asset == "USDT" {
                total_usdt_value += balance.total;
                usdt_balances.push(balance);
            }
        }
        
        Ok(AccountSummary {
            total_usdt_value,
            balances: usdt_balances,
        })
    }

    async fn get_okx_balance(&self, user_exchange: &UserExchange) -> Result<Vec<ExchangeBalance>> {
        let url = "https://www.okx.com/api/v5/account/balance";
        
        // 欧易API需要ISO8601格式的时间戳，精确到毫秒
        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let method = "GET";
        let request_path = "/api/v5/account/balance";
        let body = ""; // GET请求没有请求体
        
        // 欧易签名字符串格式: timestamp + method + requestPath + body
        let sign_string = format!("{}{}{}{}", timestamp, method, request_path, body);
        let signature = self.generate_okx_signature(&sign_string, &user_exchange.secret_key);
        
        // 添加调试日志
        log::info!("欧易API调试信息:");
        log::info!("  时间戳: {}", timestamp);
        log::info!("  签名字符串: '{}'", sign_string);
        log::info!("  API Key: {}", &user_exchange.api_key);
        log::info!("  签名结果: {}", signature);
        
        let passphrase = user_exchange.passphrase.as_ref()
            .ok_or_else(|| anyhow::anyhow!("欧易API需要passphrase"))?;
        
        // 欧易API的passphrase直接使用原始值，不需要编码
        log::info!("  Passphrase: {}", passphrase);

        let response = self.client
            .get(url)
            .header("OK-ACCESS-KEY", &user_exchange.api_key)
            .header("OK-ACCESS-SIGN", &signature)
            .header("OK-ACCESS-TIMESTAMP", &timestamp)
            .header("OK-ACCESS-PASSPHRASE", passphrase)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("欧易API调用失败: {}", response.status());
        }

        let balance_response: OkxAccountBalance = response.json().await?;
        
        if balance_response.code != "0" {
            anyhow::bail!("欧易API返回错误: {}", balance_response.msg);
        }

        let balances: Vec<ExchangeBalance> = balance_response.data
            .into_iter()
            .flat_map(|data| {
                data.details.into_iter().filter_map(|detail| {
                    let eq: Decimal = detail.eq.parse().ok()?;
                    let avail_bal: Decimal = detail.availBal.parse().ok()?;
                    let frozen_bal: Decimal = detail.frozenBal.parse().ok()?;
                    
                    if eq > Decimal::ZERO {
                        Some(ExchangeBalance {
                            asset: detail.ccy,
                            free: avail_bal,
                            locked: frozen_bal,
                            total: eq,
                            usdt_value: None, // 单个币种的USDT估值暂时不计算
                        })
                    } else {
                        None
                    }
                })
            })
            .collect();

        Ok(balances)
    }

    async fn get_okx_summary(&self, user_exchange: &UserExchange) -> Result<AccountSummary> {
        let url = "https://www.okx.com/api/v5/account/balance";
        
        // 欧易API需要ISO8601格式的时间戳，精确到毫秒
        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let method = "GET";
        let request_path = "/api/v5/account/balance";
        let body = "";
        
        let sign_string = format!("{}{}{}{}", timestamp, method, request_path, body);
        let signature = self.generate_okx_signature(&sign_string, &user_exchange.secret_key);
        
        log::info!("欧易API调试信息:");
        log::info!("  时间戳: {}", timestamp);
        log::info!("  签名字符串: '{}'", sign_string);
        log::info!("  API Key: {}", &user_exchange.api_key);
        log::info!("  签名结果: {}", signature);
        
        let passphrase = user_exchange.passphrase.as_ref()
            .ok_or_else(|| anyhow::anyhow!("欧易API需要passphrase"))?;
        
        log::info!("  Passphrase: {}", passphrase);

        let response = self.client
            .get(url)
            .header("OK-ACCESS-KEY", &user_exchange.api_key)
            .header("OK-ACCESS-SIGN", &signature)
            .header("OK-ACCESS-TIMESTAMP", &timestamp)
            .header("OK-ACCESS-PASSPHRASE", passphrase)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("欧易API调用失败: {}", response.status());
        }

        let balance_response: OkxAccountBalance = response.json().await?;
        
        if balance_response.code != "0" {
            anyhow::bail!("欧易API返回错误: {}", balance_response.msg);
        }

        let mut total_usdt_value = Decimal::ZERO;
        let mut usdt_balances = Vec::new();

        for data in balance_response.data {
            // 只处理USDT余额，忽略其他币种
            for detail in data.details {
                if detail.ccy == "USDT" {
                    let eq: Decimal = detail.eq.parse().unwrap_or_default();
                    let avail_bal: Decimal = detail.availBal.parse().unwrap_or_default();
                    let frozen_bal: Decimal = detail.frozenBal.parse().unwrap_or_default();
                    
                    if eq > Decimal::ZERO {
                        total_usdt_value += eq;
                        usdt_balances.push(ExchangeBalance {
                            asset: detail.ccy,
                            free: avail_bal,
                            locked: frozen_bal,
                            total: eq,
                            usdt_value: Some(eq), // USDT的估值就是它自身
                        });
                    }
                }
            }
        }

        Ok(AccountSummary {
            total_usdt_value,
            balances: usdt_balances,
        })
    }

    async fn get_weex_balance(&self, user_exchange: &UserExchange) -> Result<Vec<ExchangeBalance>> {
        // 根据官方文档使用正确的API端点
        let url = "https://contract-openapi.weex.com/api/spot/v1/account/assets";
        let timestamp = chrono::Utc::now().timestamp_millis();
        let method = "GET";
        let path = "/api/spot/v1/account/assets";
        
        // WEEX签名算法：根据官方文档
        // timestamp + method.toUpperCase() + requestPath + queryString
        // 对于GET请求，queryString为空字符串
        let query_string = "";
        let sign_string = format!("{}{}{}{}", timestamp, method.to_uppercase(), path, query_string);
        let signature = self.generate_weex_signature(&sign_string, &user_exchange.secret_key);

        log::info!("WEEX API调试信息:");
        log::info!("  URL: {}", url);
        log::info!("  时间戳: {}", timestamp);
        log::info!("  签名字符串: '{}'", sign_string);
        log::info!("  API Key: {}", &user_exchange.api_key);
        log::info!("  签名结果: {}", &signature);
        log::info!("  Passphrase: {}", &user_exchange.passphrase.as_deref().unwrap_or(""));

        let response = self.client
            .get(url)
            .header("ACCESS-KEY", &user_exchange.api_key)
            .header("ACCESS-SIGN", &signature)
            .header("ACCESS-PASSPHRASE", user_exchange.passphrase.as_deref().unwrap_or(""))
            .header("ACCESS-TIMESTAMP", &timestamp.to_string())
            .header("Content-Type", "application/json")
            .header("User-Agent", "BNHBot/1.0")
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("WEEX API调用失败: {} - {}", status, error_text);
        }

        let account_info: WeexAccountInfo = response.json().await?;
        
        if account_info.code != "00000" {
            anyhow::bail!("WEEX API返回错误: {}", account_info.msg);
        }

        let balances: Vec<ExchangeBalance> = account_info.data
            .into_iter()
            .filter_map(|b| {
                let free: Decimal = b.available.parse().ok()?;
                let locked: Decimal = b.frozen.parse().ok()?;
                let total: Decimal = b.equity.parse().ok()?;
                
                if total > Decimal::ZERO {
                    Some(ExchangeBalance {
                        asset: b.coin_name,
                        free,
                        locked,
                        total,
                        usdt_value: None,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(balances)
    }

    async fn get_weex_summary(&self, user_exchange: &UserExchange) -> Result<AccountSummary> {
        let balances = self.get_weex_balance(user_exchange).await?;
        
        // 只统计USDT余额，忽略其他币种
        let mut total_usdt_value = Decimal::ZERO;
        let mut usdt_balances = Vec::new();
        
        for balance in balances {
            if balance.asset == "USDT" {
                total_usdt_value += balance.total;
                usdt_balances.push(balance);
            }
        }
        
        Ok(AccountSummary {
            total_usdt_value,
            balances: usdt_balances,
        })
    }

    // 获取币种的USDT估值（通过币安价格API）
    #[allow(dead_code)]
    async fn get_usdt_value(&self, asset: &str, amount: Decimal) -> Decimal {
        if amount == Decimal::ZERO {
            return Decimal::ZERO;
        }

        // USDT直接返回
        if asset == "USDT" {
            return amount;
        }

        // 通过币安价格API获取实时价格
        match self.get_binance_price(asset).await {
            Ok(price) => amount * price,
            Err(_) => {
                // 如果获取价格失败，使用备用静态价格
                log::warn!("获取{}价格失败，使用备用价格", asset);
                let fallback_price = match asset {
                    "BTC" => Decimal::from(100000),
                    "ETH" => Decimal::from(4000),
                    "BNB" => Decimal::from(600),
                    "SOL" => Decimal::from(200),
                    "DOGE" => Decimal::new(3, 1), // 0.3
                    _ => Decimal::ZERO,
                };
                amount * fallback_price
            }
        }
    }

    // 通过币安API获取币种USDT价格
    #[allow(dead_code)]
    async fn get_binance_price(&self, asset: &str) -> Result<Decimal> {
        let symbol = format!("{}USDT", asset);
        let url = format!("https://api.binance.com/api/v3/ticker/price?symbol={}", symbol);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("币安价格API调用失败: {}", response.status());
        }

        let price_data: serde_json::Value = response.json().await?;
        let price_str = price_data["price"].as_str()
            .ok_or_else(|| anyhow::anyhow!("价格数据格式错误"))?;
        
        let price: Decimal = price_str.parse()
            .map_err(|_| anyhow::anyhow!("价格解析失败: {}", price_str))?;

        Ok(price)
    }

    fn generate_binance_signature(&self, query_string: &str, secret_key: &str) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        
        let mut mac = Hmac::<Sha256>::new_from_slice(secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(query_string.as_bytes());
        
        hex::encode(mac.finalize().into_bytes())
    }

    fn generate_okx_signature(&self, sign_string: &str, secret_key: &str) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        use base64::Engine;
        
        // 欧易API的secret_key直接使用原始字符串，不需要base64解码
        let mut mac = Hmac::<Sha256>::new_from_slice(secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(sign_string.as_bytes());
        
        base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
    }

    fn generate_weex_signature(&self, message: &str, secret_key: &str) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        
        let mut mac = Hmac::<Sha256>::new_from_slice(secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(message.as_bytes());
        
        // WEEX使用Base64编码，不是hex编码
        base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
    }
}
