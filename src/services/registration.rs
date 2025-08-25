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

        // 检查用户名是否已存在
        if self.database.is_username_exists(&request.user_name).await? {
            anyhow::bail!("用户名 '{}' 已存在，请选择其他用户名", request.user_name);
        }

        // 验证API信息
        self.validate_api_info(&request)?;

        // 解析身份类型
        let identity = match request.identity.as_deref().unwrap_or("regular").to_lowercase().as_str() {
            "student" => crate::models::registration::UserIdentity::Student,
            "regular" | _ => crate::models::registration::UserIdentity::Regular,
        };

        // 创建报名记录
        let registration = Registration {
            id,
            user_name: request.user_name,
            exchange,
            api_key: request.api_key,
            secret_key: request.secret_key,
            passphrase: request.passphrase,
            status: RegistrationStatus::Pending,
            admin_notes: None,
            identity,
            created_at: now,
            updated_at: now,
            reviewed_at: None,
        };

        // 保存到数据库
        self.database.create_registration(&registration).await?;
        
        Ok(registration)
    }

    /// 获取待审核的报名
    pub async fn get_pending_registrations(&self) -> Result<Vec<Registration>> {
        self.database.get_registrations_by_status(RegistrationStatus::Pending).await
    }

    /// 审核报名
    pub async fn review_registration(
        &self,
        registration_id: Uuid,
        status: RegistrationStatus,
        admin_notes: Option<String>,
    ) -> Result<Registration> {
        // 从数据库获取现有记录
        let mut registration = self.database.get_registration_by_id(registration_id).await?
            .ok_or_else(|| anyhow::anyhow!("报名记录不存在"))?;

        // 更新状态和备注
        registration.status = status;
        registration.admin_notes = admin_notes;
        registration.updated_at = Utc::now();
        registration.reviewed_at = Some(Utc::now());

        // 保存到数据库
        self.database.update_registration(&registration).await?;

        Ok(registration)
    }

    /// 获取报名统计
    pub async fn get_registration_stats(&self) -> Result<crate::models::registration::RegistrationStats> {
        let all_registrations = self.database.get_all_registrations().await?;
        
        let total = all_registrations.len();
        let pending = all_registrations.iter().filter(|r| matches!(r.status, RegistrationStatus::Pending)).count();
        let approved = all_registrations.iter().filter(|r| matches!(r.status, RegistrationStatus::Approved)).count();
        let rejected = all_registrations.iter().filter(|r| matches!(r.status, RegistrationStatus::Rejected)).count();
        
        let mut by_exchange = HashMap::new();
        for registration in &all_registrations {
            let exchange_key = match registration.exchange {
                RegistrationExchangeType::Binance => "binance",
                RegistrationExchangeType::OKX => "okx",
                RegistrationExchangeType::WEEX => "weex",
            };
            *by_exchange.entry(exchange_key.to_string()).or_insert(0) += 1;
        }
        
        Ok(crate::models::registration::RegistrationStats {
            total,
            pending,
            approved,
            rejected,
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
        self.database.get_all_registrations().await
    }

    /// 根据ID获取报名记录
    pub async fn get_registration_by_id(&self, id: Uuid) -> Result<Option<Registration>> {
        self.database.get_registration_by_id(id).await
    }

    /// 删除报名记录
    pub async fn delete_registration(&self, registration_id: Uuid) -> Result<()> {
        self.database.delete_registration(registration_id).await
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
