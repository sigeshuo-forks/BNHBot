use crate::models::{
    Registration, RegistrationForm, RegistrationReview, RegistrationStatus, 
    RegistrationType, RegistrationStats, User
};
use crate::services::DatabaseService;
use anyhow::Result;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

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
        user_id: Uuid,
        form: RegistrationForm,
    ) -> Result<Registration> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        // 这里应该调用数据库服务创建报名记录
        // 暂时返回模拟数据
        Ok(Registration {
            id,
            user_id,
            registration_type: form.registration_type,
            title: form.title,
            content: form.content,
            status: RegistrationStatus::Pending,
            admin_notes: None,
            admin_id: None,
            created_at: now,
            updated_at: now,
            reviewed_at: None,
        })
    }

    /// 获取用户的报名记录
    pub async fn get_user_registrations(&self, user_id: Uuid) -> Result<Vec<Registration>> {
        // 这里应该从数据库查询
        // 暂时返回空向量
        Ok(vec![])
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
        review: RegistrationReview,
    ) -> Result<Registration> {
        // 这里应该更新数据库
        // 暂时返回模拟数据
        Ok(Registration {
            id: review.registration_id,
            user_id: Uuid::new_v4(), // 临时ID
            registration_type: RegistrationType::Exchange,
            title: "临时标题".to_string(),
            content: "临时内容".to_string(),
            status: review.status,
            admin_notes: review.admin_notes,
            admin_id: Some(review.admin_id),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            reviewed_at: Some(Utc::now()),
        })
    }

    /// 获取报名统计
    pub async fn get_registration_stats(&self) -> Result<RegistrationStats> {
        // 这里应该从数据库统计
        // 暂时返回模拟数据
        Ok(RegistrationStats {
            total: 0,
            pending: 0,
            approved: 0,
            rejected: 0,
            cancelled: 0,
            by_type: HashMap::new(),
        })
    }

    /// 验证报名表单
    pub fn validate_form(&self, form: &RegistrationForm) -> Result<()> {
        if form.title.trim().is_empty() {
            anyhow::bail!("标题不能为空");
        }
        if form.content.trim().is_empty() {
            anyhow::bail!("内容不能为空");
        }
        if form.contact_info.trim().is_empty() {
            anyhow::bail!("联系信息不能为空");
        }
        Ok(())
    }

    /// 生成报名链接
    pub fn generate_registration_url(&self, registration_id: Uuid) -> String {
        // 这里应该生成实际的Web报名链接
        format!("https://your-domain.com/register/{}", registration_id)
    }

    /// 检查用户是否有权限报名
    pub async fn check_user_permission(&self, user_id: Uuid) -> Result<bool> {
        // 这里应该检查用户权限
        // 暂时返回true
        Ok(true)
    }
}
