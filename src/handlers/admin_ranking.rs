use axum::{extract::State, response::Json, http::StatusCode};
use serde::{Serialize, Deserialize};
use crate::services::{RegistrationService, AuthService, ExchangeService, RankingService, DingTalkBot};
use log::{info, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendRankingResponse {
    pub success: bool,
    pub message: String,
}

/// 手动发送排名到钉钉群（管理员功能）
pub async fn send_ranking_to_dingtalk(
    State((_, _, _, ranking_service, dingtalk_bot)): State<(RegistrationService, AuthService, ExchangeService, RankingService, DingTalkBot)>,
) -> Result<Json<SendRankingResponse>, StatusCode> {
    info!("管理员手动触发发送排名到钉钉群");
    
    match send_ranking_notification(&ranking_service, &dingtalk_bot).await {
        Ok(message) => {
            info!("手动发送排名成功: {}", message);
            Ok(Json(SendRankingResponse {
                success: true,
                message,
            }))
        }
        Err(e) => {
            error!("手动发送排名失败: {}", e);
            Ok(Json(SendRankingResponse {
                success: false,
                message: format!("发送失败: {}", e),
            }))
        }
    }
}

async fn send_ranking_notification(
    ranking_service: &RankingService,
    dingtalk_bot: &DingTalkBot,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // 获取排名数据
    let rankings_response = ranking_service.get_rankings().await?;
    
    // 检查是否有数据
    if rankings_response.daily_rankings.is_empty() {
        return Ok("暂无排名数据，未发送消息".to_string());
    }
    
    // 生成钉钉排名消息
    let ranking_url = "http://localhost:3000/rankings"; // TODO: 从配置获取
    let ranking_message = ranking_service.get_dingtalk_ranking_message(ranking_url).await?;
    
    // 格式化消息内容
    let mut message_content = String::new();
    message_content.push_str("🏆 交易大赛实时排名播报\n\n");
    message_content.push_str(&format!("📊 参赛人数: {} 人\n", ranking_message.total_participants));
    message_content.push_str("🎯 前5名排名:\n\n");
    
    for (index, entry) in ranking_message.top_rankings.iter().enumerate() {
        let rank_icon = match index + 1 {
            1 => "🥇",
            2 => "🥈", 
            3 => "🥉",
            _ => "🏅",
        };
        
        let trend_icon = if entry.change_percentage >= rust_decimal::Decimal::ZERO {
            "📈"
        } else {
            "📉"
        };
        
        let doubled_badge = if entry.is_doubled { " 🚀翻倍" } else { "" };
        
        message_content.push_str(&format!(
            "{} #{} {} - {}%{}\n💰 ${} ({}${})\n\n",
            rank_icon,
            entry.rank,
            entry.user_name,
            if entry.change_percentage >= rust_decimal::Decimal::ZERO { "+" } else { "" },
            entry.change_percentage,
            doubled_badge,
            entry.current_balance,
            if entry.change_amount >= rust_decimal::Decimal::ZERO { "+" } else { "" },
            entry.change_amount.abs()
        ));
    }
    
    message_content.push_str(&format!("📈 查看完整排名: {}\n", ranking_message.ranking_url));
    message_content.push_str("💪 继续加油，争取更好成绩！");
    
    // 发送到钉钉
    dingtalk_bot.send_text_message(&message_content).await?;
    
    Ok(format!("成功发送排名到钉钉群，包含{}名参赛者的前5名排名", ranking_message.total_participants))
}
