use anyhow::Result;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;

/// JWT Claims结构
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // 主题 (通常是用户ID)
    pub exp: usize,   // 过期时间
    pub iat: usize,   // 签发时间
    pub role: String, // 角色
}

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

/// 登录响应
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    pub token: Option<String>,
    pub expires_at: Option<String>,
}

/// 认证服务
#[derive(Clone)]
pub struct AuthService {
    jwt_secret: String,
    admin_password_hash: String,
    session_timeout_hours: i64,
}

impl AuthService {
    pub fn new() -> Result<Self> {
        let jwt_secret = env::var("JWT_SECRET")
            .unwrap_or_else(|_| "default_jwt_secret_change_in_production".to_string());

        let admin_password = env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin123".to_string());

        let session_timeout_hours = env::var("SESSION_TIMEOUT_HOURS")
            .unwrap_or_else(|_| "24".to_string())
            .parse()
            .unwrap_or(24);

        // 对管理员密码进行哈希
        let admin_password_hash = Self::hash_password(&admin_password);

        log::info!("🔐 认证服务初始化完成");
        log::info!("🕐 会话超时时间: {} 小时", session_timeout_hours);

        Ok(Self {
            jwt_secret,
            admin_password_hash,
            session_timeout_hours,
        })
    }

    /// 验证管理员登录
    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse> {
        // 验证密码
        let password_hash = Self::hash_password(&request.password);

        if password_hash != self.admin_password_hash {
            log::warn!("🚨 管理员登录失败: 密码错误");
            return Ok(LoginResponse {
                success: false,
                message: "密码错误".to_string(),
                token: None,
                expires_at: None,
            });
        }

        // 生成JWT token
        let now = Utc::now();
        let expires_at = now + Duration::hours(self.session_timeout_hours);

        let claims = Claims {
            sub: "admin".to_string(),
            exp: expires_at.timestamp() as usize,
            iat: now.timestamp() as usize,
            role: "admin".to_string(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )?;

        log::info!(
            "✅ 管理员登录成功，token过期时间: {}",
            expires_at.format("%Y-%m-%d %H:%M:%S")
        );

        Ok(LoginResponse {
            success: true,
            message: "登录成功".to_string(),
            token: Some(token),
            expires_at: Some(expires_at.to_rfc3339()),
        })
    }

    /// 验证JWT token
    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        log::debug!("🔍 开始验证token，长度: {}", token.len());

        let validation = Validation::new(Algorithm::HS256);

        let token_data = match decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &validation,
        ) {
            Ok(data) => {
                log::debug!("🔍 Token解码成功");
                data
            }
            Err(e) => {
                log::debug!("🔍 Token解码失败: {}", e);
                anyhow::bail!("Token格式无效: {}", e);
            }
        };

        // 检查是否过期
        let now = Utc::now().timestamp() as usize;
        log::debug!(
            "🔍 当前时间: {}, Token过期时间: {}",
            now,
            token_data.claims.exp
        );

        if token_data.claims.exp < now {
            let expired_seconds = now - token_data.claims.exp;
            log::debug!("🔍 Token已过期 {} 秒", expired_seconds);
            anyhow::bail!("Token已过期");
        }

        log::debug!(
            "🔍 Token验证成功，用户: {}, 角色: {}",
            token_data.claims.sub,
            token_data.claims.role
        );
        Ok(token_data.claims)
    }

    /// 验证是否为管理员
    pub fn verify_admin(&self, token: &str) -> Result<bool> {
        let claims = self.verify_token(token)?;
        Ok(claims.role == "admin")
    }

    /// 从请求头中提取token
    pub fn extract_token_from_header(auth_header: Option<&str>) -> Option<String> {
        auth_header
            .and_then(|header| header.strip_prefix("Bearer "))
            .map(|token| token.to_string())
    }

    /// 密码哈希函数
    fn hash_password(password: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(b"bnhbot_salt_2025"); // 添加盐值
        format!("{:x}", hasher.finalize())
    }

    /// 生成安全的随机JWT密钥
    pub fn generate_jwt_secret() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let mut hasher = Sha256::new();
        hasher.update(timestamp.to_string().as_bytes());
        hasher.update(b"bnhbot_jwt_secret_generator");
        format!("{:x}", hasher.finalize())
    }

    /// 检查密码强度
    pub fn check_password_strength(password: &str) -> (u8, String) {
        let mut score = 0u8;
        let mut feedback = Vec::new();

        if password.len() >= 8 {
            score += 1;
        } else {
            feedback.push("至少8个字符");
        }

        if password.chars().any(|c| c.is_ascii_lowercase()) {
            score += 1;
        } else {
            feedback.push("包含小写字母");
        }

        if password.chars().any(|c| c.is_ascii_uppercase()) {
            score += 1;
        } else {
            feedback.push("包含大写字母");
        }

        if password.chars().any(|c| c.is_ascii_digit()) {
            score += 1;
        } else {
            feedback.push("包含数字");
        }

        if password.chars().any(|c| !c.is_alphanumeric()) {
            score += 1;
        } else {
            feedback.push("包含特殊字符");
        }

        let strength_text = match score {
            0..=1 => "很弱",
            2 => "弱",
            3 => "一般",
            4 => "强",
            5 => "很强",
            _ => "未知",
        };

        let message = if feedback.is_empty() {
            format!("密码强度: {}", strength_text)
        } else {
            format!("密码强度: {}，建议: {}", strength_text, feedback.join("、"))
        };

        (score, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "test_password";
        let hash1 = AuthService::hash_password(password);
        let hash2 = AuthService::hash_password(password);

        // 相同密码应该产生相同哈希
        assert_eq!(hash1, hash2);

        // 不同密码应该产生不同哈希
        let different_hash = AuthService::hash_password("different_password");
        assert_ne!(hash1, different_hash);
    }

    #[test]
    fn test_password_strength() {
        let (score, _) = AuthService::check_password_strength("weak");
        assert!(score <= 2);

        let (score, _) = AuthService::check_password_strength("StrongP@ssw0rd!");
        assert!(score >= 4);
    }

    #[test]
    fn test_jwt_secret_generation() {
        let secret1 = AuthService::generate_jwt_secret();
        let secret2 = AuthService::generate_jwt_secret();

        // 每次生成的密钥应该不同
        assert_ne!(secret1, secret2);
        assert!(secret1.len() > 32); // 确保足够长
    }
}
