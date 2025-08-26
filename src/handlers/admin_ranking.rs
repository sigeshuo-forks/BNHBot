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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRankingResponse {
    pub success: bool,
    pub message: String,
    pub daily_count: usize,
    pub weekly_count: usize,
    pub monthly_count: usize,
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
                
                // 获取身份标识
                let identity_badge = match entry.identity.as_str() {
                    "Student" => " 🎓",
                    _ => "",
                };
                
                message_content.push_str(&format!(
                    "{} {}. {}{} {}{} ({})\n💰 余额: ${:.2} USDT\n{} 变化: ${:.2} ({:.2}%)\n📅 参与: {} 天\n\n",
                    rank_icon,
                    entry.rank,
                    entry.user_name,
                    identity_badge,
                    entry.user_label,
                    doubled_badge,
                    entry.exchange_type,
                    entry.current_balance,
                    change_icon,
                    entry.change_amount,
                    entry.change_percentage,
                    entry.participation_days
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

/// 手动收集余额数据并更新排名表（管理员功能）
/// 从各交易所获取最新余额数据，然后计算并更新固定排名表
pub async fn update_fixed_rankings(
    State(ranking_service): State<RankingService>,
) -> Json<UpdateRankingResponse> {
    info!("管理员手动触发排名表更新");
    
    // 触发余额收集，这会自动更新排名表
    match ranking_service.collect_all_balances().await {
        Ok(result) => {
            if result.success {
                // 获取刚刚更新的排名数据统计
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                
                // 尝试获取各周期的排名数量
                let daily_count = ranking_service.get_fixed_rankings("daily", &today).await.map(|r| r.len()).unwrap_or(0);
                let weekly_count = ranking_service.get_fixed_rankings("weekly", &today).await.map(|r| r.len()).unwrap_or(0);
                let monthly_count = ranking_service.get_fixed_rankings("monthly", &today).await.map(|r| r.len()).unwrap_or(0);
                
                info!("手动排名表更新成功");
                Json(UpdateRankingResponse {
                    success: true,
                    message: format!("排名表更新成功！余额收集: 成功 {}, 失败 {}", result.collected_count, result.failed_count),
                    daily_count,
                    weekly_count,
                    monthly_count,
                })
            } else {
                error!("排名表更新失败: {}", result.message);
                Json(UpdateRankingResponse {
                    success: false,
                    message: format!("排名表更新失败: {}", result.message),
                    daily_count: 0,
                    weekly_count: 0,
                    monthly_count: 0,
                })
            }
        }
        Err(e) => {
            error!("触发排名表更新失败: {}", e);
            Json(UpdateRankingResponse {
                success: false,
                message: format!("触发排名表更新失败: {}", e),
                daily_count: 0,
                weekly_count: 0,
                monthly_count: 0,
            })
        }
    }
}