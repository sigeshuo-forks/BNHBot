use crate::models::DailyReport;
use anyhow::Result;
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;

#[derive(Clone)]
pub struct DingTalkBot {
    webhook_url: String,
    secret: Option<String>,
    client: Client,
}

#[derive(Debug, Serialize)]
struct DingTalkMessage {
    msgtype: String,
    text: DingTalkText,
    at: DingTalkAt,
}

#[derive(Debug, Serialize)]
struct DingTalkText {
    content: String,
}

#[derive(Debug, Serialize)]
struct DingTalkAt {
    #[serde(rename = "atMobiles")]
    at_mobiles: Vec<String>,
    #[serde(rename = "isAtAll")]
    is_at_all: bool,
}

impl DingTalkBot {
    pub fn new(webhook_url: String, secret: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            webhook_url,
            secret,
            client,
        }
    }

    pub async fn send_daily_report(&self, report: &DailyReport) -> Result<()> {
        let content = self.format_daily_report(report);
        let message = DingTalkMessage {
            msgtype: "text".to_string(),
            text: DingTalkText { content },
            at: DingTalkAt {
                at_mobiles: vec![],
                is_at_all: false,
            },
        };

        self.send_message(&message).await
    }

    pub async fn send_balance_alert(&self, user_name: &str, exchange_type: &str, balance: &str) -> Result<()> {
        let content = format!("🔔 余额提醒\n用户: {}\n交易所: {}\n余额: {}", user_name, exchange_type, balance);
        let message = DingTalkMessage {
            msgtype: "text".to_string(),
            text: DingTalkText { content },
            at: DingTalkAt {
                at_mobiles: vec![],
                is_at_all: false,
            },
        };

        self.send_message(&message).await
    }

    fn format_daily_report(&self, report: &DailyReport) -> String {
        let mut content = format!("📊 每日余额播报 - {}\n", report.date);
        content.push_str(&format!("总用户数: {}\n", report.total_users));
        content.push_str(&format!("总资产价值: ${:.2}\n\n", report.total_value));

        for summary in &report.user_summaries {
            content.push_str(&format!("👤 {}\n", summary.user_name));
            content.push_str(&format!("🏢 {}\n", summary.exchange_type));
            content.push_str(&format!("💰 ${:.2}\n", summary.total_usdt_value));
            
            // 显示主要币种余额
            let top_balances: Vec<_> = summary.balances.iter()
                .filter(|b| b.usdt_value.unwrap_or_default() > rust_decimal::Decimal::new(1, 0))
                .take(5)
                .collect();
            
            for balance in top_balances {
                if let Some(usdt_value) = balance.usdt_value {
                    content.push_str(&format!("  {}: {:.4} (${:.2})\n", 
                        balance.asset, balance.total, usdt_value));
                }
            }
            content.push_str("\n");
        }

        content
    }

    async fn send_message(&self, message: &DingTalkMessage) -> Result<()> {
        let mut url = self.webhook_url.clone();
        
        // 如果有签名密钥，添加时间戳和签名
        if let Some(secret) = &self.secret {
            let timestamp = chrono::Utc::now().timestamp_millis();
            let string_to_sign = format!("{}\n{}", timestamp, secret);
            let signature = crate::utils::crypto::hmac_sha256(&string_to_sign, secret);
            url.push_str(&format!("&timestamp={}&sign={}", timestamp, signature));
        }

        let response = self.client
            .post(&url)
            .json(message)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("钉钉API调用失败: {}", error_text);
        }

        Ok(())
    }
}
