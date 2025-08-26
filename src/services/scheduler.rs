use crate::services::{DatabaseService, ExchangeService, DingTalkBot, RankingService};
use crate::utils::timezone::TimezoneUtil;
use anyhow::Result;
use chrono::{DateTime, Utc, TimeZone, Duration};
use log::{info, error};
use rust_decimal::Decimal;
use tokio::time::{sleep, Duration as TokioDuration};

#[derive(Clone)]
pub struct Scheduler {
    database: DatabaseService,
    exchange_service: ExchangeService,
    dingtalk_bot: DingTalkBot,
    ranking_service: RankingService,
}

impl Scheduler {
    pub fn new(database: DatabaseService, exchange_service: ExchangeService, dingtalk_bot: DingTalkBot, ranking_service: RankingService) -> Self {
        Self {
            database,
            exchange_service,
            dingtalk_bot,
            ranking_service,
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("启动定时任务调度器...");
        
        loop {
            let now = Utc::now();
            let next_run = self.get_next_run_time(now);
            let sleep_duration = (next_run - now).num_seconds() as u64;
            
            // 显示中国时间便于理解
            let next_run_china = TimezoneUtil::utc_to_china(next_run);
            let now_china = TimezoneUtil::utc_to_china(now);
            
            info!("当前中国时间: {}, 下次执行中国时间: {}, 等待 {} 秒", 
                  TimezoneUtil::format_china_time(now_china), 
                  TimezoneUtil::format_china_time(next_run_china), 
                  sleep_duration);
            sleep(TokioDuration::from_secs(sleep_duration)).await;
            
            if let Err(e) = self.execute_daily_task().await {
                error!("执行每日任务失败: {}", e);
            }
        }
    }

    fn get_next_run_time(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        // 转换当前UTC时间到中国时间
        let now_china = TimezoneUtil::utc_to_china(now);
        
        // 设置为中国时间每日8点执行
        let china_tz = TimezoneUtil::china_timezone();
        let target_time_china = now_china.date_naive().and_hms_opt(8, 0, 0).unwrap();
        let target_datetime_china = china_tz.from_local_datetime(&target_time_china).unwrap();
        
        // 转换回UTC时间
        let target_datetime_utc = TimezoneUtil::china_to_utc(target_datetime_china);
        
        if now >= target_datetime_utc {
            // 如果今天中国时间8点已经过了，设置为明天8点
            target_datetime_utc + Duration::days(1)
        } else {
            target_datetime_utc
        }
    }

    async fn execute_daily_task(&self) -> Result<()> {
        info!("开始执行每日余额收集和排名任务...");
        
        // 1. 收集所有用户的余额数据
        let collection_result = self.ranking_service.collect_all_balances().await?;
        info!("余额收集结果: {}", collection_result.message);
        
        if collection_result.collected_count == 0 {
            info!("没有成功收集到任何余额数据，跳过排名计算");
            return Ok(());
        }
        
        // 2. 生成钉钉排名消息（前5名）
        let ranking_url = "http://localhost:3000/rankings"; // TODO: 从配置获取
        let ranking_message = self.ranking_service.get_dingtalk_ranking_message(ranking_url).await?;
        
        // 3. 发送钉钉排名通知
        if let Err(e) = self.send_ranking_notification(&ranking_message).await {
            error!("发送钉钉排名通知失败: {}", e);
        } else {
            info!("每日排名播报发送成功");
        }
        
        Ok(())
    }

    async fn send_ranking_notification(&self, ranking_message: &crate::models::ranking::DingTalkRankingMessage) -> Result<()> {
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
            
            let change_icon = if entry.change_amount >= Decimal::ZERO { "📈" } else { "📉" };
            let doubled_badge = if entry.is_doubled { " 🚀翻倍" } else { "" };
            
            // 获取身份标识
            let identity_badge = match entry.identity.as_str() {
                "Student" => " 🎓",
                _ => "",
            };
            
            message_content.push_str(&format!(
                "{} {}. {}{} {} ({})\n💰 余额: ${:.2} USDT\n{} 变化: ${:.2} ({:.2}%){}\n📅 参与: {} 天\n\n",
                rank_icon,
                entry.rank,
                entry.user_name,
                identity_badge,
                entry.user_label,
                entry.exchange_type,
                entry.current_balance,
                change_icon,
                entry.change_amount,
                entry.change_percentage,
                doubled_badge,
                entry.participation_days
            ));
        }
        
        message_content.push_str("📈 查看详细排名和图表:\n");
        message_content.push_str(&format!("🔗 {}\n\n", ranking_message.ranking_url));
        message_content.push_str("💪 继续加油，期待明日翻仓的你！");
        
        self.dingtalk_bot.send_text_message(&message_content).await?;
        Ok(())
    }

    // 手动触发余额查询（用于测试）
    pub async fn trigger_balance_query(&self) -> Result<()> {
        info!("手动触发余额查询...");
        self.execute_daily_task().await
    }
}
