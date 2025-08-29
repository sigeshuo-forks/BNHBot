use crate::services::{DingTalkBot, RankingService};
use axum::{extract::State, response::Json};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
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

    match ranking_service
        .get_dingtalk_ranking_message(&ranking_url)
        .await
    {
        Ok(ranking_message) => {
            // 构建钉钉消息内容
            let mut message_content = String::new();
            message_content.push_str("🏆 每日交易大赛排名榜\n\n");
            message_content.push_str(&format!(
                "📊 参赛人数: {} 人\n",
                ranking_message.total_participants
            ));
            message_content.push_str("🥇 前5名排名:\n\n");

            for (index, entry) in ranking_message.top_rankings.iter().enumerate() {
                let rank_icon = match index {
                    0 => "🥇",
                    1 => "🥈",
                    2 => "🥉",
                    _ => "🏅",
                };

                let change_icon = if entry.change_amount >= rust_decimal::Decimal::ZERO {
                    "📈"
                } else {
                    "📉"
                };
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
                        message: format!(
                            "成功发送排名到钉钉群！共发送前{}名排名。",
                            ranking_message.top_rankings.len()
                        ),
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
                let daily_count = ranking_service
                    .get_fixed_rankings("daily", &today)
                    .await
                    .map(|r| r.len())
                    .unwrap_or(0);
                let weekly_count = ranking_service
                    .get_fixed_rankings("weekly", &today)
                    .await
                    .map(|r| r.len())
                    .unwrap_or(0);
                let monthly_count = ranking_service
                    .get_fixed_rankings("monthly", &today)
                    .await
                    .map(|r| r.len())
                    .unwrap_or(0);

                info!("手动排名表更新成功");
                Json(UpdateRankingResponse {
                    success: true,
                    message: format!(
                        "排名表更新成功！余额收集: 成功 {}, 失败 {}",
                        result.collected_count, result.failed_count
                    ),
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

/// 手动触发每小时排名更新（管理员功能）
pub async fn trigger_hourly_update(
    State((ranking_service, dingtalk_bot)): State<(RankingService, DingTalkBot)>,
) -> Json<SendRankingResponse> {
    info!("管理员手动触发每小时排名更新");

    // 1. 收集所有用户的余额数据
    info!("🔄 开始收集所有用户余额数据...");
    match ranking_service.collect_all_balances().await {
        Ok(collection_result) => {
            info!("✅ 余额收集完成 - 结果: {}", collection_result.message);
            info!(
                "📊 收集统计: 成功 {} 个, 失败 {} 个",
                collection_result.collected_count, collection_result.failed_count
            );

            if collection_result.collected_count == 0 {
                return Json(SendRankingResponse {
                    success: false,
                    message: "没有成功收集到任何余额数据".to_string(),
                });
            }
        }
        Err(e) => {
            error!("收集余额数据失败: {}", e);
            return Json(SendRankingResponse {
                success: false,
                message: format!("收集余额数据失败: {}", e),
            });
        }
    }

    // 2. 更新排名表
    info!("🔄 开始更新排名表...");
    match ranking_service.update_fixed_rankings_public().await {
        Ok(_) => {
            info!("✅ 排名表更新完成");
        }
        Err(e) => {
            error!("更新排名表失败: {}", e);
            return Json(SendRankingResponse {
                success: false,
                message: format!("更新排名表失败: {}", e),
            });
        }
    }

    // 3. 检查是否有新的翻仓用户
    info!("🎉 开始检查翻仓用户...");
    match ranking_service.get_rankings().await {
        Ok(rankings) => {
            let daily_rankings = rankings.daily_rankings;
            let mut doubled_count = 0;

            for entry in daily_rankings {
                if entry.is_doubled {
                    doubled_count += 1;
                    info!(
                        "🎉 发现翻仓用户: {} ({})",
                        entry.user_name, entry.exchange_type
                    );

                    // 发送祝贺消息
                    if let Err(e) = send_congratulation_message(&dingtalk_bot, &entry).await {
                        error!("发送翻仓祝贺消息失败: {}", e);
                    } else {
                        info!("✅ 翻仓祝贺消息发送成功: {}", entry.user_name);
                    }
                }
            }

            if doubled_count > 0 {
                info!("🎉 本次更新发现 {} 个翻仓用户", doubled_count);
            }
        }
        Err(e) => {
            warn!("检查翻仓用户失败: {}", e);
        }
    }

    info!("✅ 每小时排名更新完成");
    Json(SendRankingResponse {
        success: true,
        message: "每小时排名更新完成！已检查翻仓用户并发送祝贺消息。".to_string(),
    })
}

/// 手动触发Top5播报（管理员功能）
pub async fn trigger_top5_broadcast(
    State((ranking_service, dingtalk_bot)): State<(RankingService, DingTalkBot)>,
) -> Json<SendRankingResponse> {
    info!("管理员手动触发Top5播报");

    // 获取基础URL配置
    let base_url = env::var("WEB_SERVER_BASE_URL")
        .or_else(|_| env::var("WEB_BASE_URL"))
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    let ranking_url = format!("{}/rankings", base_url);

    match ranking_service
        .get_dingtalk_ranking_message(&ranking_url)
        .await
    {
        Ok(ranking_message) => {
            // 构建钉钉Top5播报消息内容
            let mut message_content = String::new();
            message_content.push_str("🏆 每日交易大赛排名榜 (手动播报)\n\n");
            message_content.push_str(&format!(
                "📊 参赛人数: {} 人\n",
                ranking_message.total_participants
            ));
            message_content.push_str("🥇 前5名排名:\n\n");

            for (index, entry) in ranking_message.top_rankings.iter().enumerate() {
                let rank_icon = match index {
                    0 => "🥇",
                    1 => "🥈",
                    2 => "🥉",
                    _ => "🏅",
                };

                let change_icon = if entry.change_amount >= rust_decimal::Decimal::ZERO {
                    "📈"
                } else {
                    "📉"
                };
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
                    info!("手动Top5播报成功");
                    Json(SendRankingResponse {
                        success: true,
                        message: format!(
                            "成功发送Top5播报到钉钉群！共发送前{}名排名。",
                            ranking_message.top_rankings.len()
                        ),
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

/// 手动检查翻仓祝贺（管理员功能）
pub async fn check_congratulations(
    State((ranking_service, dingtalk_bot)): State<(RankingService, DingTalkBot)>,
) -> Json<SendRankingResponse> {
    info!("管理员手动检查翻仓祝贺");

    match ranking_service.get_rankings().await {
        Ok(rankings) => {
            let daily_rankings = rankings.daily_rankings;

            if daily_rankings.is_empty() {
                return Json(SendRankingResponse {
                    success: false,
                    message: "没有排名数据".to_string(),
                });
            }

            let mut doubled_count = 0;
            let mut congratulated_count = 0;

            for entry in daily_rankings {
                if entry.is_doubled {
                    doubled_count += 1;
                    info!(
                        "🎉 发现翻仓用户: {} ({})",
                        entry.user_name, entry.exchange_type
                    );

                    // 发送祝贺消息
                    match send_congratulation_message(&dingtalk_bot, &entry).await {
                        Ok(_) => {
                            congratulated_count += 1;
                            info!("✅ 翻仓祝贺消息发送成功: {}", entry.user_name);
                        }
                        Err(e) => {
                            error!("发送翻仓祝贺消息失败: {}", e);
                        }
                    }
                }
            }

            if doubled_count == 0 {
                Json(SendRankingResponse {
                    success: true,
                    message: "当前没有翻仓用户".to_string(),
                })
            } else {
                Json(SendRankingResponse {
                    success: true,
                    message: format!(
                        "发现 {} 个翻仓用户，成功发送 {} 条祝贺消息",
                        doubled_count, congratulated_count
                    ),
                })
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

/// 发送翻仓祝贺消息的辅助函数
async fn send_congratulation_message(
    dingtalk_bot: &DingTalkBot,
    entry: &crate::models::ranking::RankingEntry,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut message_content = String::new();
    message_content.push_str("🎉🎉🎉 恭喜翻仓！🎉🎉🎉\n\n");
    message_content.push_str(&format!(
        "🚀 用户: {} ({})\n",
        entry.user_name, entry.exchange_type
    ));
    message_content.push_str(&format!(
        "💰 当前余额: ${:.2} USDT\n",
        entry.current_balance
    ));
    message_content.push_str(&format!("📈 累积收益率: {:.2}%\n", entry.change_percentage));
    message_content.push_str(&format!("📅 参与天数: {} 天\n", entry.participation_days));

    // 获取身份标识
    let identity_badge = match entry.identity.as_str() {
        "Student" => "🎓 学员",
        _ => "👤 普通用户",
    };
    message_content.push_str(&format!("👑 身份: {}\n\n", identity_badge));

    // 获取基础URL配置
    let base_url = env::var("WEB_SERVER_BASE_URL")
        .or_else(|_| env::var("WEB_BASE_URL"))
        .unwrap_or_else(|_| "http://localhost:3000".to_string());

    message_content.push_str("🏆 查看详细排名:\n");
    message_content.push_str(&format!("🔗 {}/rankings\n\n", base_url));
    message_content.push_str("💪 恭喜你实现翻仓目标！继续保持，再创佳绩！");

    dingtalk_bot.send_text_message(&message_content).await?;
    Ok(())
}
