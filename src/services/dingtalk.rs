use crate::models::DailyReport;
use anyhow::Result;
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::time::Duration;
use log::info;

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

#[derive(Debug, Deserialize)]
struct DingTalkErrorResponse {
    errcode: i32,
    errmsg: String,
}

#[derive(Debug, Serialize)]
struct DingTalkText {
    content: String,
}

#[derive(Debug, Serialize)]
struct DingTalkAt {
    #[serde(rename = "atMobiles")]
    at_mobiles: Vec<String>,
    #[serde(rename = "atUserIds")]
    at_user_ids: Vec<String>,
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
                at_user_ids: vec![],
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
                at_user_ids: vec![],
                is_at_all: false,
            },
        };

        self.send_message(&message).await
    }

    /// 发送文本消息
    pub async fn send_text_message(&self, message: &str) -> Result<()> {
        let payload = DingTalkMessage {
            msgtype: "text".to_string(),
            text: DingTalkText {
                content: message.to_string(),
            },
            at: DingTalkAt {
                at_mobiles: vec![],
                at_user_ids: vec![],
                is_at_all: false,
            },
        };
        
        self.send_message(&payload).await
    }

    /// 发送带@功能的文本消息
    pub async fn send_text_message_with_at(&self, message: &str, at_mobiles: Option<Vec<String>>, at_user_ids: Option<Vec<String>>) -> Result<()> {
        let at = DingTalkAt {
            at_mobiles: at_mobiles.unwrap_or_default(),
            at_user_ids: at_user_ids.unwrap_or_default(),
            is_at_all: false,
        };

        let payload = DingTalkMessage {
            msgtype: "text".to_string(),
            text: DingTalkText {
                content: message.to_string(),
            },
            at,
        };
        
        self.send_message(&payload).await
    }

    /// 发送启动通知
    pub async fn send_startup_notification(&self, at_all: bool) -> Result<()> {
        let startup_time = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        
        // 获取系统信息
        let system_info = self.get_system_info();
        
        let content = format!(
            "🤖 BNHBot 钉钉机器人已启动\n\n\
            🕐 启动时间: {}\n\
            💻 系统信息: {}\n\
            ✅ 系统状态: 正常运行\n\n\
            🎯 主要功能:\n\
            • 📊 每日8点自动播报交易所余额并进行排名播报\n\
            • 🏢 支持币安、欧易、WEEX三大交易所\n\n\
            💡 使用方法:\n\
            • 输入 \"@机器人 报名\" 进行报名\n\
            • 系统会自动发送报名链接\n\n\
            📞 如有问题，请联系管理员 @西柚",
            startup_time, system_info
        );

        let message = DingTalkMessage {
            msgtype: "text".to_string(),
            text: DingTalkText { content },
            at: DingTalkAt {
                at_mobiles: vec![],
                at_user_ids: vec![],
                is_at_all: at_all,
            },
        };

        self.send_message(&message).await
    }

    /// 发送报名通知给管理员
    pub async fn send_registration_notification(
        &self,
        user_name: &str,
        registration_type: &str,
        title: &str,
        admin_webhook: &str,
    ) -> Result<()> {
        let content = format!(
            "📝 新报名通知\n用户: {}\n类型: {}\n标题: {}\n\n请点击链接进行审核：{}",
            user_name, registration_type, title, admin_webhook
        );
        
        let message = DingTalkMessage {
            msgtype: "text".to_string(),
            text: DingTalkText { content },
            at: DingTalkAt {
                at_mobiles: vec![],
                at_user_ids: vec![],
                is_at_all: false,
            },
        };

        self.send_message(&message).await
    }

    /// 发送报名状态更新通知
    pub async fn send_registration_status_update(
        &self,
        user_name: &str,
        title: &str,
        status: &str,
        notes: Option<&str>,
    ) -> Result<()> {
        let mut content = format!(
            "📋 报名状态更新\n标题: {}\n状态: {}\n",
            title, status
        );
        
        if let Some(notes) = notes {
            content.push_str(&format!("备注: {}", notes));
        }

        let message = DingTalkMessage {
            msgtype: "text".to_string(),
            text: DingTalkText { content },
            at: DingTalkAt {
                at_mobiles: vec![],
                at_user_ids: vec![],
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
            // 钉钉签名算法：timestamp + \n + secret
            let string_to_sign = format!("{}\n{}", timestamp, secret);
            // 使用base64编码的签名，而不是hex编码
            let signature = crate::utils::crypto::hmac_sha256_base64(&string_to_sign, secret);
            url.push_str(&format!("&timestamp={}&sign={}", timestamp, signature));
            
            info!("签名信息 - 时间戳: {}, 签名字符串: '{}', 签名结果: {}", timestamp, string_to_sign, signature);
        }

        info!("发送钉钉消息到: {}", url);
        info!("消息内容: {:?}", message);
        
        let response = self.client
            .post(&url)
            .json(message)
            .send()
            .await?;

        let status = response.status();
        let headers = response.headers().clone();
        
        // 获取响应体
        let response_text = response.text().await.unwrap_or_default();
        
        info!("钉钉API响应 - 状态码: {}, 响应体: {}", status, response_text);
        
        if !status.is_success() {
            // 尝试解析钉钉的错误响应
            if let Ok(error_response) = serde_json::from_str::<DingTalkErrorResponse>(&response_text) {
                anyhow::bail!(
                    "钉钉API调用失败 - 状态码: {}, 错误码: {}, 错误信息: {}",
                    status, error_response.errcode, error_response.errmsg
                );
            } else {
                anyhow::bail!(
                    "钉钉API调用失败 - 状态码: {}, 响应体: {}, 响应头: {:?}",
                    status, response_text, headers
                );
            }
        }
        
        // 即使状态码成功，也要检查钉钉的业务错误码
        if let Ok(api_response) = serde_json::from_str::<DingTalkErrorResponse>(&response_text) {
            if api_response.errcode != 0 {
                let error_description = self.get_error_description(api_response.errcode);
                anyhow::bail!(
                    "钉钉API业务错误 - 错误码: {}, 错误信息: {}, 错误说明: {}",
                    api_response.errcode, api_response.errmsg, error_description
                );
            }
            info!("钉钉API调用成功 - 错误码: {}, 错误信息: {}", api_response.errcode, api_response.errmsg);
        }

        Ok(())
    }

    /// 获取系统信息
    fn get_system_info(&self) -> String {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let rust_version = env!("CARGO_PKG_VERSION");
        
        format!("{}-{} (Rust {})", os, arch, rust_version)
    }

    /// 获取钉钉API错误码说明
    fn get_error_description(&self, errcode: i32) -> String {
        match errcode {
            0 => "成功".to_string(),
            1 => "系统繁忙，请稍后重试".to_string(),
            2 => "参数错误".to_string(),
            3 => "无效的access_token".to_string(),
            4 => "access_token过期".to_string(),
            5 => "access_token无效".to_string(),
            6 => "access_token无效".to_string(),
            7 => "access_token无效".to_string(),
            8 => "access_token无效".to_string(),
            9 => "access_token无效".to_string(),
            10 => "access_token无效".to_string(),
            310000 => "机器人发送签名不匹配 - 请检查签名密钥配置".to_string(),
            _ => format!("未知错误码: {}", errcode),
        }
    }
}
