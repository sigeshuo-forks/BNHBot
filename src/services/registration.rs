use crate::models::registration::{Registration, RegistrationRequest, RegistrationStatus, RegistrationExchangeType};
use crate::services::DatabaseService;
use anyhow::Result;
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;

#[derive(Clone)]
pub struct RegistrationService {
    database: DatabaseService,
}

impl RegistrationService {
    pub fn new(database: DatabaseService) -> Self {
        Self { database }
    }

    /// 创建新报名
    pub async fn create_registration(
        &self,
        request: RegistrationRequest,
    ) -> Result<Registration> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        // 解析交易所类型
        let exchange = match request.exchange.to_lowercase().as_str() {
            "binance" => RegistrationExchangeType::Binance,
            "okx" => RegistrationExchangeType::OKX,
            "weex" => RegistrationExchangeType::WEEX,
            _ => anyhow::bail!("不支持的交易所类型: {}", request.exchange),
        };

        // 验证API信息
        self.validate_api_info(&request)?;

        // 这里应该调用数据库服务创建报名记录
        // 暂时返回模拟数据
        Ok(Registration {
            id,
            exchange,
            api_key: request.api_key,
            secret_key: request.secret_key,
            passphrase: request.passphrase,
            status: RegistrationStatus::Pending,
            admin_notes: None,
            created_at: now,
            updated_at: now,
            reviewed_at: None,
        })
    }

    /// 获取待审核的报名
    pub async fn get_pending_registrations(&self) -> Result<Vec<Registration>> {
        // 这里应该从数据库查询
        // 暂时返回空向量
        Ok(vec![])
    }

    /// 审核报名
    pub async fn review_registration(
        &self,
        review: crate::models::registration::RegistrationReview,
    ) -> Result<Registration> {
        // 这里应该更新数据库
        // 暂时返回模拟数据
        Ok(Registration {
            id: review.registration_id,
            exchange: RegistrationExchangeType::Binance, // 临时值
            api_key: "temp_key".to_string(),
            secret_key: "temp_secret".to_string(),
            passphrase: None,
            status: review.status,
            admin_notes: review.admin_notes,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            reviewed_at: Some(Utc::now()),
        })
    }

    /// 获取报名统计
    pub async fn get_registration_stats(&self) -> Result<crate::models::registration::RegistrationStats> {
        // 这里应该从数据库统计
        // 暂时返回模拟数据
        Ok(crate::models::registration::RegistrationStats {
            total: 0,
            pending: 0,
            approved: 0,
            rejected: 0,
            by_exchange: HashMap::new(),
        })
    }

    /// 验证API信息
    fn validate_api_info(&self, request: &RegistrationRequest) -> Result<()> {
        if request.api_key.trim().is_empty() {
            anyhow::bail!("API Key不能为空");
        }
        if request.secret_key.trim().is_empty() {
            anyhow::bail!("Secret Key不能为空");
        }
        
        // 如果是OKX，必须提供Passphrase
        if request.exchange.to_lowercase() == "okx" && request.passphrase.as_ref().map_or(true, |p| p.trim().is_empty()) {
            anyhow::bail!("欧易(OKX)必须提供Passphrase");
        }

        Ok(())
    }

    /// 测试API连接
    pub async fn test_api_connection(&self, _request: &RegistrationRequest) -> Result<bool> {
        // 这里应该实际测试API连接
        // 暂时返回true表示测试通过
        Ok(true)
    }

    /// 获取交易所API要求说明
    pub fn get_exchange_requirements(exchange: &str) -> String {
        match exchange.to_lowercase().as_str() {
            "binance" => "币安需要API Key和Secret Key，请确保API具有读取权限".to_string(),
            "okx" => "欧易需要API Key、Secret Key和Passphrase，请确保API具有读取权限".to_string(),
            "weex" => "WEEX需要API Key和Secret Key，请确保API具有读取权限".to_string(),
            _ => "未知交易所".to_string(),
        }
    }
}
