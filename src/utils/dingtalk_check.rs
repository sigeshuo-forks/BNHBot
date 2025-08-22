use crate::utils::crypto::hmac_sha256_base64;
use chrono::Utc;

/// 钉钉机器人配置检查工具
pub struct DingTalkChecker;

impl DingTalkChecker {
    /// 检查签名配置
    pub fn check_signature_config(webhook_url: &str, secret: &str) -> String {
        let timestamp = Utc::now().timestamp_millis();
        let string_to_sign = format!("{}\n{}", timestamp, secret);
        let signature = hmac_sha256_base64(&string_to_sign, secret);
        
        format!(
            "🔍 钉钉机器人签名配置检查\n\n\
            📋 配置信息:\n\
            • Webhook URL: {}\n\
            • 签名密钥: {}\n\
            • 当前时间戳: {}\n\
            • 签名字符串: '{}'\n\
            • 生成的签名: {}\n\n\
            🔗 完整URL:\n\
            {}&timestamp={}&sign={}\n\n\
            💡 检查要点:\n\
            • 确保签名密钥与钉钉机器人设置中的密钥完全一致\n\
            • 确保时间戳使用毫秒级精度\n\
            • 确保签名字符串格式为: timestamp\\nsecret\n\
            • 确保签名使用base64编码",
            webhook_url, secret, timestamp, string_to_sign, signature,
            webhook_url, timestamp, signature
        )
    }
    
    /// 验证签名是否正确
    pub fn verify_signature(webhook_url: &str, secret: &str, timestamp: i64, signature: &str) -> bool {
        let string_to_sign = format!("{}\n{}", timestamp, secret);
        let expected_signature = hmac_sha256_base64(&string_to_sign, secret);
        
        signature == expected_signature
    }
}
