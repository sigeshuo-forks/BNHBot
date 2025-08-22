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

        // 验证用户名称
        if request.user_name.trim().is_empty() {
            anyhow::bail!("用户名称不能为空");
        }

        // 验证API信息
        self.validate_api_info(&request)?;

        // 这里应该调用数据库服务创建报名记录
        // 暂时返回模拟数据
        Ok(Registration {
            id,
            user_name: request.user_name,
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
        registration_id: Uuid,
        status: RegistrationStatus,
        admin_notes: Option<String>,
    ) -> Result<Registration> {
        // 这里应该更新数据库
        // 暂时返回模拟数据
        Ok(Registration {
            id: registration_id,
            user_name: "临时用户".to_string(), // 临时值
            exchange: RegistrationExchangeType::Binance, // 临时值
            api_key: "temp_key".to_string(),
            secret_key: "temp_secret".to_string(),
            passphrase: None,
            status,
            admin_notes,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            reviewed_at: Some(Utc::now()),
        })
    }

    /// 获取报名统计
    pub async fn get_registration_stats(&self) -> Result<crate::models::registration::RegistrationStats> {
        // 这里应该从数据库统计
        // 暂时返回模拟数据
        let mut by_exchange = HashMap::new();
        by_exchange.insert("binance".to_string(), 1);
        by_exchange.insert("okx".to_string(), 1);
        by_exchange.insert("weex".to_string(), 1);
        
        Ok(crate::models::registration::RegistrationStats {
            total: 3,
            pending: 1,
            approved: 1,
            rejected: 1,
            by_exchange,
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

    /// 获取所有报名记录（管理员用）
    pub async fn get_all_registrations(&self) -> Result<Vec<Registration>> {
        // 这里应该从数据库查询所有报名记录
        // 暂时返回模拟数据
        let now = Utc::now();
        Ok(vec![
            Registration {
                id: Uuid::new_v4(),
                user_name: "张三".to_string(),
                exchange: RegistrationExchangeType::Binance,
                api_key: "BNBXXXXXXXXXXXXX".to_string(),
                secret_key: "secret123456".to_string(),
                passphrase: None,
                status: RegistrationStatus::Pending,
                admin_notes: None,
                created_at: now,
                updated_at: now,
                reviewed_at: None,
            },
            Registration {
                id: Uuid::new_v4(),
                user_name: "李四".to_string(),
                exchange: RegistrationExchangeType::OKX,
                api_key: "OKXAPIKEY123".to_string(),
                secret_key: "okxsecret456".to_string(),
                passphrase: Some("okxpass789".to_string()),
                status: RegistrationStatus::Approved,
                admin_notes: Some("API验证通过".to_string()),
                created_at: now,
                updated_at: now,
                reviewed_at: Some(now),
            },
            Registration {
                id: Uuid::new_v4(),
                user_name: "王五".to_string(),
                exchange: RegistrationExchangeType::WEEX,
                api_key: "WEEXKEY789".to_string(),
                secret_key: "weexsecret123".to_string(),
                passphrase: None,
                status: RegistrationStatus::Rejected,
                admin_notes: Some("API无效".to_string()),
                created_at: now,
                updated_at: now,
                reviewed_at: Some(now),
            },
        ])
    }

    /// 删除报名记录
    pub async fn delete_registration(&self, registration_id: Uuid) -> Result<()> {
        // 这里应该从数据库删除报名记录
        // 暂时模拟成功
        log::info!("删除报名记录: {}", registration_id);
        Ok(())
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
