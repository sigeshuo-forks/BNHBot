use log::{info, error};
use std::collections::HashMap;

/// 发送审核通过通知到钉钉群
pub async fn send_approval_notification(
    dingtalk_bot: crate::services::DingTalkBot,
    database: crate::services::DatabaseService,
    user_name: String,
    exchange_type: String,
) {
    info!("发送审核通过通知: 用户 {} ({})", user_name, exchange_type);
    
    // 获取统计信息
    let stats_result = get_member_statistics(&database).await;
    
    let (total_members, exchange_stats) = match stats_result {
        Ok((total, stats)) => (total, stats),
        Err(e) => {
            error!("获取统计信息失败: {}", e);
            (0, HashMap::new())
        }
    };
    
    // 构建通知消息
    let exchange_display = match exchange_type.to_lowercase().as_str() {
        "binance" => "币安 Binance",
        "okx" => "欧易 OKX", 
        "weex" => "WEEX",
        _ => &exchange_type,
    };
    
    let mut message = String::new();
    message.push_str("🎉 新成员加入交易大赛！\n\n");
    message.push_str(&format!("👤 新成员: {}\n", user_name));
    message.push_str(&format!("🏢 交易所: {}\n", exchange_display));
    message.push_str(&format!("📊 当前总人数: {} 人\n\n", total_members));
    
    // 添加交易所分布统计
    if !exchange_stats.is_empty() {
        message.push_str("📈 交易所分布:\n");
        for (exchange, count) in exchange_stats {
            let exchange_name = match exchange.as_str() {
                "Binance" | "binance" => "币安",
                "OKX" | "okx" => "欧易",
                "WEEX" | "weex" => "WEEX",
                _ => &exchange,
            };
            message.push_str(&format!("• {}: {} 人\n", exchange_name, count));
        }
        message.push_str("\n");
    }
    
    message.push_str("🚀 欢迎新成员！期待您在交易大赛中的精彩表现！\n");
    message.push_str("💪 让我们一起见证财富增长的奇迹！");
    
    // 发送钉钉消息
    match dingtalk_bot.send_text_message(&message).await {
        Ok(_) => {
            info!("审核通过通知发送成功: {}", user_name);
        }
        Err(e) => {
            error!("发送审核通过通知失败: {}", e);
        }
    }
}

/// 获取成员统计信息
async fn get_member_statistics(
    database: &crate::services::DatabaseService,
) -> Result<(usize, HashMap<String, usize>), anyhow::Error> {
    // 获取所有已通过审核的用户
    let approved_registrations = database.get_registrations_by_status(
        crate::models::registration::RegistrationStatus::Approved
    ).await?;
    
    let total_members = approved_registrations.len();
    
    // 统计各交易所人数
    let mut exchange_stats = HashMap::new();
    for registration in approved_registrations {
        let exchange_key = registration.exchange.to_string();
        *exchange_stats.entry(exchange_key).or_insert(0) += 1;
    }
    
    Ok((total_members, exchange_stats))
}
