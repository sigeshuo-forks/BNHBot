use crate::models::{RegistrationForm, RegistrationType, RegistrationStatus};
use crate::services::{RegistrationService, DingTalkBot};
use anyhow::Result;
use clap::{Parser, Subcommand};
use log::info;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "registration")]
#[command(about = "报名系统管理")]
pub struct RegistrationCli {
    #[command(subcommand)]
    command: RegistrationCommands,
}

#[derive(Subcommand)]
pub enum RegistrationCommands {
    /// 创建新报名
    Create {
        /// 钉钉用户ID
        #[arg(long)]
        dingtalk_id: String,
        /// 报名类型 (exchange/event/training/other)
        #[arg(long)]
        registration_type: String,
        /// 标题
        #[arg(long)]
        title: String,
        /// 内容
        #[arg(long)]
        content: String,
        /// 联系信息
        #[arg(long)]
        contact_info: String,
    },
    /// 查询用户报名记录
    Query {
        /// 钉钉用户ID
        #[arg(long)]
        dingtalk_id: String,
    },
    /// 审核报名
    Review {
        /// 报名ID
        #[arg(long)]
        registration_id: String,
        /// 审核状态 (approved/rejected/cancelled)
        #[arg(long)]
        status: String,
        /// 管理员备注
        #[arg(long)]
        admin_notes: Option<String>,
        /// 管理员ID
        #[arg(long)]
        admin_id: String,
    },
    /// 获取待审核报名列表
    Pending,
    /// 获取报名统计
    Stats,
}

pub struct RegistrationHandler {
    registration_service: RegistrationService,
    dingtalk_bot: DingTalkBot,
}

impl RegistrationHandler {
    pub fn new(registration_service: RegistrationService, dingtalk_bot: DingTalkBot) -> Self {
        Self {
            registration_service,
            dingtalk_bot,
        }
    }

    pub async fn handle(&self, cli: RegistrationCli) -> Result<()> {
        match cli.command {
            RegistrationCommands::Create {
                dingtalk_id,
                registration_type,
                title,
                content,
                contact_info,
            } => {
                self.create_registration(&dingtalk_id, &registration_type, &title, &content, &contact_info).await
            }
            RegistrationCommands::Query { dingtalk_id } => {
                self.query_user_registrations(&dingtalk_id).await
            }
            RegistrationCommands::Review {
                registration_id,
                status,
                admin_notes,
                admin_id,
            } => {
                self.review_registration(&registration_id, &status, admin_notes.as_deref(), &admin_id).await
            }
            RegistrationCommands::Pending => {
                self.get_pending_registrations().await
            }
            RegistrationCommands::Stats => {
                self.get_registration_stats().await
            }
        }
    }

    async fn create_registration(
        &self,
        dingtalk_id: &str,
        registration_type: &str,
        title: &str,
        content: &str,
        contact_info: &str,
    ) -> Result<()> {
        info!("创建新报名: {} ({})", title, dingtalk_id);

        // 解析报名类型
        let reg_type = match registration_type.to_lowercase().as_str() {
            "exchange" => RegistrationType::Exchange,
            "event" => RegistrationType::Event,
            "training" => RegistrationType::Training,
            "other" => RegistrationType::Other,
            _ => {
                println!("无效的报名类型: {}", registration_type);
                return Ok(());
            }
        };

        // 创建报名表单
        let form = RegistrationForm {
            registration_type: reg_type,
            title: title.to_string(),
            content: content.to_string(),
            contact_info: contact_info.to_string(),
            additional_fields: None,
        };

        // 验证表单
        if let Err(e) = self.registration_service.validate_form(&form) {
            println!("表单验证失败: {}", e);
            return Ok(());
        }

        // 创建报名记录
        let registration = self.registration_service.create_registration(
            Uuid::new_v4(), // 临时用户ID，实际应该从数据库查询
            form,
        ).await?;

        println!("报名创建成功！");
        println!("报名ID: {}", registration.id);
        println!("状态: {}", registration.status);
        println!("创建时间: {}", registration.created_at);

        // 发送通知给管理员
        let admin_webhook = self.registration_service.generate_registration_url(registration.id);
        if let Err(e) = self.dingtalk_bot.send_registration_notification(
            "用户", // 实际应该获取用户名
            &registration.registration_type.to_string(),
            &registration.title,
            &admin_webhook,
        ).await {
            println!("发送管理员通知失败: {}", e);
        }

        Ok(())
    }

    async fn query_user_registrations(&self, dingtalk_id: &str) -> Result<()> {
        info!("查询用户报名记录: {}", dingtalk_id);

        // 这里应该先查询用户ID，然后查询报名记录
        let user_id = Uuid::new_v4(); // 临时ID
        let registrations = self.registration_service.get_user_registrations(user_id).await?;

        if registrations.is_empty() {
            println!("用户 {} 暂无报名记录", dingtalk_id);
            return Ok(());
        }

        println!("用户 {} 的报名记录:", dingtalk_id);
        for reg in registrations {
            println!("- ID: {}", reg.id);
            println!("  类型: {}", reg.registration_type);
            println!("  标题: {}", reg.title);
            println!("  状态: {}", reg.status);
            println!("  创建时间: {}", reg.created_at);
            println!();
        }

        Ok(())
    }

    async fn review_registration(
        &self,
        registration_id: &str,
        status: &str,
        admin_notes: Option<&str>,
        admin_id: &str,
    ) -> Result<()> {
        info!("审核报名: {} -> {}", registration_id, status);

        // 解析状态
        let review_status = match status.to_lowercase().as_str() {
            "approved" => RegistrationStatus::Approved,
            "rejected" => RegistrationStatus::Rejected,
            "cancelled" => RegistrationStatus::Cancelled,
            _ => {
                println!("无效的审核状态: {}", status);
                return Ok(());
            }
        };

        // 解析报名ID
        let reg_id = match Uuid::parse_str(registration_id) {
            Ok(id) => id,
            Err(_) => {
                println!("无效的报名ID: {}", registration_id);
                return Ok(());
            }
        };

        // 解析管理员ID
        let admin_uuid = match Uuid::parse_str(admin_id) {
            Ok(id) => id,
            Err(_) => {
                println!("无效的管理员ID: {}", admin_id);
                return Ok(());
            }
        };

        // 执行审核
        let review = crate::models::RegistrationReview {
            registration_id: reg_id,
            status: review_status,
            admin_notes: admin_notes.map(|s| s.to_string()),
            admin_id: admin_uuid,
        };

        let updated_registration = self.registration_service.review_registration(review).await?;

        println!("报名审核完成！");
        println!("报名ID: {}", updated_registration.id);
        println!("新状态: {}", updated_registration.status);
        println!("审核时间: {}", updated_registration.reviewed_at.unwrap());

        // 发送状态更新通知
        if let Err(e) = self.dingtalk_bot.send_registration_status_update(
            "用户", // 实际应该获取用户名
            &updated_registration.title,
            &updated_registration.status.to_string(),
            updated_registration.admin_notes.as_deref(),
        ).await {
            println!("发送状态更新通知失败: {}", e);
        }

        Ok(())
    }

    async fn get_pending_registrations(&self) -> Result<()> {
        info!("获取待审核报名列表");

        let registrations = self.registration_service.get_pending_registrations().await?;

        if registrations.is_empty() {
            println!("暂无待审核的报名");
            return Ok(());
        }

        println!("待审核报名列表:");
        for reg in registrations {
            println!("- ID: {}", reg.id);
            println!("  用户ID: {}", reg.user_id);
            println!("  类型: {}", reg.registration_type);
            println!("  标题: {}", reg.title);
            println!("  创建时间: {}", reg.created_at);
            println!();
        }

        Ok(())
    }

    async fn get_registration_stats(&self) -> Result<()> {
        info!("获取报名统计");

        let stats = self.registration_service.get_registration_stats().await?;

        println!("报名统计:");
        println!("总数量: {}", stats.total);
        println!("待审核: {}", stats.pending);
        println!("已通过: {}", stats.approved);
        println!("已拒绝: {}", stats.rejected);
        println!("已取消: {}", stats.cancelled);

        if !stats.by_type.is_empty() {
            println!("\n按类型统计:");
            for (reg_type, count) in &stats.by_type {
                println!("  {}: {}", reg_type, count);
            }
        }

        Ok(())
    }
}
