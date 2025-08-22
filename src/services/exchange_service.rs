use crate::models::{ExchangeType, ExchangeBalance, UserExchange};
use crate::models::exchange::*;
use anyhow::Result;
use reqwest::Client;
use rust_decimal::Decimal;
use std::time::Duration;

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
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(balances)
    }

    async fn get_okx_balance(&self, user_exchange: &UserExchange) -> Result<Vec<ExchangeBalance>> {
        let url = "https://www.okx.com/api/v5/account/balance";
        
        let timestamp = chrono::Utc::now().to_rfc3339();
        let method = "GET";
        let request_path = "/api/v5/account/balance";
        
        let sign_string = format!("{}{}{}", timestamp, method, request_path);
        let signature = self.generate_okx_signature(&sign_string, &user_exchange.secret_key);
        
        let passphrase = user_exchange.passphrase.as_ref()
            .ok_or_else(|| anyhow::anyhow!("欧易API需要passphrase"))?;

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
                        })
                    } else {
                        None
                    }
                })
            })
            .collect();

        Ok(balances)
    }

    async fn get_weex_balance(&self, user_exchange: &UserExchange) -> Result<Vec<ExchangeBalance>> {
        let url = "https://api.weex.com/v1/account";
        let timestamp = chrono::Utc::now().timestamp_millis();
        
        let signature = self.generate_weex_signature(
            &format!("timestamp={}", timestamp),
            &user_exchange.secret_key,
        );

        let response = self.client
            .get(url)
            .header("X-WEEX-APIKEY", &user_exchange.api_key)
            .header("X-WEEX-SIGNATURE", &signature)
            .header("X-WEEX-TIMESTAMP", &timestamp.to_string())
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("WEEX API调用失败: {}", response.status());
        }

        let account_info: WeexAccountInfo = response.json().await?;
        
        if account_info.code != 0 {
            anyhow::bail!("WEEX API返回错误: {}", account_info.msg);
        }

        let balances: Vec<ExchangeBalance> = account_info.data.balances
            .into_iter()
            .filter_map(|b| {
                let free: Decimal = b.free.parse().ok()?;
                let locked: Decimal = b.locked.parse().ok()?;
                let total: Decimal = b.total.parse().ok()?;
                
                if total > Decimal::ZERO {
                    Some(ExchangeBalance {
                        asset: b.asset,
                        free,
                        locked,
                        total,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(balances)
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
        
        hex::encode(mac.finalize().into_bytes())
    }
}
