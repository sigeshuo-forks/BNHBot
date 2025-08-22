use serde_json::json;
use std::collections::HashMap;

/// Webhook测试工具
pub struct WebhookTester;

impl WebhookTester {
    /// 生成测试用的钉钉消息
    pub fn generate_test_message() -> serde_json::Value {
        json!({
            "msgtype": "text",
            "text": {
                "content": "报名"
            },
            "at": {
                "atMobiles": ["13800138000"],
                "atUserIds": ["user123"],
                "isAtAll": false
            }
        })
    }
    
    /// 生成包含@的测试消息
    pub fn generate_at_message() -> serde_json::Value {
        json!({
            "msgtype": "text",
            "text": {
                "content": "@机器人 报名"
            },
            "at": {
                "atMobiles": ["13800138000"],
                "atUserIds": ["user123"],
                "isAtAll": false
            }
        })
    }
    
    /// 生成普通测试消息
    pub fn generate_plain_message() -> serde_json::Value {
        json!({
            "msgtype": "text",
            "text": {
                "content": "你好机器人"
            },
            "at": {
                "atMobiles": [],
                "atUserIds": [],
                "isAtAll": false
            }
        })
    }
    
    /// 生成测试用的curl命令
    pub fn generate_curl_commands(webhook_url: &str) -> Vec<String> {
        let test_messages = vec![
            ("报名消息", Self::generate_test_message()),
            ("@机器人消息", Self::generate_at_message()),
            ("普通消息", Self::generate_plain_message()),
        ];
        
        test_messages
            .into_iter()
            .map(|(name, message)| {
                format!(
                    "# 测试: {}\ncurl -X POST {} \\\n  -H 'Content-Type: application/json' \\\n  -d '{}'",
                    name,
                    webhook_url,
                    serde_json::to_string_pretty(&message).unwrap()
                )
            })
            .collect()
    }
}
