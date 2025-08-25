use axum::{extract::State, response::Json};
use serde::{Serialize, Deserialize};
use crate::services::{RankingService, DingTalkBot};
use log::{info, error};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendRankingResponse {
    pub success: bool,
    pub message: String,
}

/// 手动发送排名到钉钉群（管理员功能）
/// 仅基于数据库中已有的余额数据计算排名并发送，不重新获取交易所数据
pub async fn send_ranking_to_dingtalk(
    State((ranking_service, dingtalk_bot)): State<(RankingService, DingTalkBot)>,
) -> Json<SendRankingResponse> {
    info!("管理员手动发送排名到钉钉群（基于已有数据）");
    
    // 获取基础URL配置
    let base_url = env::var("WEB_SERVER_BASE_URL")
        .or_else(|_| env::var("WEB_BASE_URL"))
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    let ranking_url = format!("{}/rankings", base_url);
    
    match ranking_service.get_dingtalk_ranking_message(&ranking_url).await {
        Ok(ranking_message) => {
            // 构建钉钉消息内容
            let mut message_content = String::new();
            message_content.push_str("🏆 每日交易大赛排名榜\n\n");
            message_content.push_str(&format!("📊 参赛人数: {} 人\n", ranking_message.total_participants));
            message_content.push_str("🥇 前5名排名:\n\n");
            
            for (index, entry) in ranking_message.top_rankings.iter().enumerate() {
                let rank_icon = match index {
                    0 => "🥇",
                    1 => "🥈", 
                    2 => "🥉",
                    _ => "🏅",
                };
                
                let change_icon = if entry.change_amount >= rust_decimal::Decimal::ZERO { "📈" } else { "📉" };
                let doubled_badge = if entry.is_doubled { " 🚀翻倍" } else { "" };
                
                message_content.push_str(&format!(
                    "{} {}. {} {}{} ({})\n💰 余额: ${:.2} USDT\n{} 变化: ${:.2} ({:.2}%)\n\n",
                    rank_icon,
                    entry.rank,
                    entry.user_name,
                    entry.user_label,
                    doubled_badge,
                    entry.exchange_type,
                    entry.current_balance,
                    change_icon,
                    entry.change_amount,
                    entry.change_percentage
                ));
            }
            
            message_content.push_str("📈 查看详细排名和图表:\n");
            message_content.push_str(&format!("🔗 {}\n\n", ranking_message.ranking_url));
            message_content.push_str("💪 继续加油，期待明日翻仓的你！");
            
            // 发送钉钉消息
            match dingtalk_bot.send_text_message(&message_content).await {
                Ok(_) => {
                    info!("手动发送排名成功");
                    Json(SendRankingResponse {
                        success: true,
                        message: format!("成功发送排名到钉钉群！共发送前{}名排名。", ranking_message.top_rankings.len()),
                    })
                }
                Err(e) => {
                    error!("发送钉钉消息失败: {}", e);
                    Json(SendRankingResponse {
                        success: false,
                        message: format!("发送钉钉消息失败: {}", e),
                    })
                }
            }
        }
        Err(e) => {
            error!("获取排名数据失败: {}", e);
            Json(SendRankingResponse {
                success: false,
                message: format!("获取排名数据失败: {}", e),
            })
        }
    }
}