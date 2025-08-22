use crate::services::{DatabaseService, DingTalkBot};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use log::{info, error};

#[derive(Debug, Deserialize)]
pub struct DingTalkMessage {
    pub msgtype: String,
    pub text: Option<DingTalkText>,
    pub at: Option<DingTalkAt>,
}

#[derive(Debug, Deserialize)]
pub struct DingTalkText {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct DingTalkAt {
    #[serde(rename = "atMobiles")]
    pub at_mobiles: Vec<String>,
    #[serde(rename = "atUserIds")]
    pub at_user_ids: Vec<String>,
    #[serde(rename = "isAtAll")]
    pub is_at_all: bool,
}

#[derive(Debug, Deserialize)]
pub struct DingTalkUser {
    pub userid: String,
    pub name: String,
    pub mobile: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DingTalkResponse {
    pub errcode: i32,
    pub errmsg: String,
}

#[derive(Clone)]
pub struct DingTalkWebhookHandler {
    database: DatabaseService,
    dingtalk_bot: DingTalkBot,
    web_base_url: String,
}

impl DingTalkWebhookHandler {
    pub fn new(database: DatabaseService, dingtalk_bot: DingTalkBot, web_base_url: String) -> Self {
        Self {
            database,
            dingtalk_bot,
            web_base_url,
        }
    }

    /// 检查钉钉机器人配置
    pub async fn check_config(&self) -> Result<()> {
        info!("🔧 检查钉钉机器人配置...");
        
        // 发送测试消息
        let test_message = "🤖 钉钉机器人连接测试\n✅ 配置正常，可以接收消息";
        self.dingtalk_bot.send_text_message(test_message).await?;
        
        info!("✅ 钉钉机器人配置检查完成");
        Ok(())
    }

    pub async fn handle_message(&self, message: DingTalkMessage) -> Result<DingTalkResponse> {
        info!("收到钉钉消息: {:?}", message);
        
        // 检查消息类型
        if message.msgtype != "text" {
            info!("非文本消息，忽略");
            return Ok(DingTalkResponse {
                errcode: 0,
                errmsg: "success".to_string(),
            });
        }

        let text_content = match &message.text {
            Some(text) => text.content.trim(),
            None => {
                info!("消息内容为空");
                return Ok(DingTalkResponse {
                    errcode: 0,
                    errmsg: "success".to_string(),
                });
            },
        };

        info!("收到文本消息: '{}'", text_content);

        // 检查是否包含"报名"关键词
        if text_content.contains("报名") {
            info!("检测到报名请求: {}", text_content);
            
            // 获取@的用户信息
            let at_users = self.extract_at_users(&message).await?;
            info!("@的用户数量: {}", at_users.len());
            
            if at_users.is_empty() {
                // 如果没有@用户，发送通用回复
                let registration_url = format!("{}/register", self.web_base_url);
                let reply_message = format!(
                    "🎯 您好！\n\n📝 请点击以下链接进行报名：\n🔗 {}\n\n💡 报名说明：\n• 支持多种报名类型\n• 填写完成后自动提交审核\n• 管理员会及时处理您的申请\n\n💡 操作步骤：\n1. 点击上方报名链接\n2. 填写报名信息\n3. 提交等待审核\n\n❓ 如有问题，请联系管理员",
                    registration_url
                );
                
                self.dingtalk_bot.send_text_message(&reply_message).await?;
                info!("发送通用报名回复");
            } else {
                // 为每个@的用户发送报名链接
                for user in at_users {
                    self.send_registration_link(&user).await?;
                }
            }
            
            return Ok(DingTalkResponse {
                errcode: 0,
                errmsg: "success".to_string(),
            });
        }

        // 其他消息，返回成功
        info!("消息不包含'报名'关键词，忽略");
        Ok(DingTalkResponse {
            errcode: 0,
            errmsg: "success".to_string(),
        })
    }

    async fn extract_at_users(&self, message: &DingTalkMessage) -> Result<Vec<DingTalkUser>> {
        let mut users = Vec::new();
        
        if let Some(at) = &message.at {
            // 处理@的手机号
            for mobile in &at.at_mobiles {
                if let Ok(user) = self.get_user_by_mobile(mobile).await {
                    users.push(user);
                }
            }
            
            // 处理@的用户ID
            for userid in &at.at_user_ids {
                if let Ok(user) = self.get_user_by_id(userid).await {
                    users.push(user);
                }
            }
        }
        
        Ok(users)
    }

    async fn get_user_by_mobile(&self, mobile: &str) -> Result<DingTalkUser> {
        // 这里应该从数据库查询用户信息
        // 暂时返回模拟数据
        Ok(DingTalkUser {
            userid: "unknown".to_string(),
            name: "用户".to_string(),
            mobile: Some(mobile.to_string()),
        })
    }

    async fn get_user_by_id(&self, userid: &str) -> Result<DingTalkUser> {
        // 这里应该从数据库查询用户信息
        // 暂时返回模拟数据
        Ok(DingTalkUser {
            userid: userid.to_string(),
            name: "用户".to_string(),
            mobile: None,
        })
    }

    async fn send_registration_link(&self, user: &DingTalkUser) -> Result<()> {
        let web_url = "http://localhost:3000/register";
        
        let message = format!(
            "🎯 @{} 您好！\n\n\
            📝 请点击以下链接进行报名：\n\
            🔗 {}\n\n\
            💡 报名说明：\n\
            • 支持多种报名类型\n\
            • 填写完成后自动提交审核\n\
            • 管理员会及时处理您的申请\n\n\
            ❓ 如有问题，请联系管理员",
            user.name, web_url
        );

        // 发送钉钉消息，@用户
        if let Some(mobile) = &user.mobile {
            self.dingtalk_bot.send_text_message_with_at(
                &message,
                Some(vec![mobile.clone()]),
                None
            ).await?;
        } else {
            self.dingtalk_bot.send_text_message(&message).await?;
        }
        
        info!("已向用户 {} 发送报名链接", user.name);
        Ok(())
    }
}
