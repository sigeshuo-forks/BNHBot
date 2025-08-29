use crate::services::{DatabaseService, DingTalkBot, ExchangeService, RankingService};
use crate::utils::timezone::TimezoneUtil;
use anyhow::Result;
use chrono::{DateTime, Duration, TimeZone, Timelike, Utc};
use log::{error, info, warn};
use rust_decimal::Decimal;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::time::{sleep, Duration as TokioDuration};

#[derive(Clone)]
pub struct Scheduler {
    database: DatabaseService,
    exchange_service: ExchangeService,
    dingtalk_bot: DingTalkBot,
    ranking_service: RankingService,
    web_base_url: String,
    // 记录已经祝贺过的翻仓用户，避免重复发送
    congratulated_users: Arc<tokio::sync::Mutex<HashSet<String>>>,
}

impl Scheduler {
    pub fn new(
        database: DatabaseService,
        exchange_service: ExchangeService,
        dingtalk_bot: DingTalkBot,
        ranking_service: RankingService,
        web_base_url: String,
    ) -> Self {
        Self {
            database,
            exchange_service,
            dingtalk_bot,
            ranking_service,
            web_base_url,
            congratulated_users: Arc::new(tokio::sync::Mutex::new(HashSet::new())),
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("启动定时任务调度器...");

        // 启动多个定时任务
        let scheduler_clone = self.clone();
        let hourly_scheduler = scheduler_clone.clone();
        let daily_scheduler = scheduler_clone.clone();
        let congratulation_scheduler = scheduler_clone.clone();

        // 1. 每小时排名更新任务
        tokio::spawn(async move {
            if let Err(e) = hourly_scheduler.start_hourly_ranking_update().await {
                error!("每小时排名更新任务失败: {}", e);
            }
        });

        // 2. 每日晚上8点播报Top5排名任务
        tokio::spawn(async move {
            if let Err(e) = daily_scheduler.start_daily_top5_broadcast().await {
                error!("每日Top5播报任务失败: {}", e);
            }
        });

        // 3. 翻仓用户即时祝贺任务
        tokio::spawn(async move {
            if let Err(e) = congratulation_scheduler
                .start_congratulation_monitor()
                .await
            {
                error!("翻仓祝贺监控任务失败: {}", e);
            }
        });

        info!("✅ 所有定时任务已启动");

        // 主线程不阻塞，让后台任务独立运行
        Ok(())
    }

    // 每小时排名更新任务
    async fn start_hourly_ranking_update(&self) -> Result<()> {
        info!("🔄 启动每小时排名更新任务...");

        loop {
            let now = Utc::now();

            // 计算到下一个整点的秒数
            let current_minute = now.minute() as u64;
            let current_second = now.second() as u64;
            let sleep_duration = (60 - current_minute) * 60 - current_second;

            // 计算下一个整点时间（用于显示）
            let next_hour = (now.hour() + 1) % 24;
            let next_hour_china = if next_hour == 0 {
                // 跨天的情况
                let tomorrow = now.date_naive() + chrono::Duration::days(1);
                let next_time = tomorrow.and_hms_opt(0, 0, 0).unwrap();
                let next_utc = Utc.from_local_datetime(&next_time).unwrap();
                TimezoneUtil::utc_to_china(next_utc)
            } else {
                // 同一天的情况
                let next_time = now.date_naive().and_hms_opt(next_hour, 0, 0).unwrap();
                let next_utc = Utc.from_local_datetime(&next_time).unwrap();
                TimezoneUtil::utc_to_china(next_utc)
            };

            info!(
                "⏰ 下次排名更新时间: {} (中国时间), 等待 {} 秒",
                TimezoneUtil::format_china_time(next_hour_china),
                sleep_duration
            );

            sleep(TokioDuration::from_secs(sleep_duration)).await;

            if let Err(e) = self.execute_hourly_ranking_update().await {
                error!("执行每小时排名更新失败: {}", e);
            }
        }
    }

    // 每日晚上8点播报Top5排名任务
    async fn start_daily_top5_broadcast(&self) -> Result<()> {
        info!("📢 启动每日晚上8点Top5播报任务...");

        loop {
            let now = Utc::now();
            let next_run = self.get_next_daily_broadcast_time(now);
            let sleep_duration = (next_run - now).num_seconds() as u64;

            info!(
                "⏰ 下次Top5播报时间: {} (中国时间), 等待 {} 秒",
                TimezoneUtil::format_china_time(TimezoneUtil::utc_to_china(next_run)),
                sleep_duration
            );

            sleep(TokioDuration::from_secs(sleep_duration)).await;

            if let Err(e) = self.execute_daily_top5_broadcast().await {
                error!("执行每日Top5播报失败: {}", e);
            }
        }
    }

    // 翻仓用户即时祝贺监控任务
    async fn start_congratulation_monitor(&self) -> Result<()> {
        info!("🎉 启动翻仓用户即时祝贺监控任务...");

        loop {
            // 每5分钟检查一次是否有新的翻仓用户
            tokio::time::sleep(TokioDuration::from_secs(300)).await;

            if let Err(e) = self.check_and_congratulate_doubled_users().await {
                error!("检查翻仓用户失败: {}", e);
            }
        }
    }

    // 每小时排名更新执行
    async fn execute_hourly_ranking_update(&self) -> Result<()> {
        info!("🔄 开始执行每小时排名更新...");

        // 1. 收集所有用户的余额数据
        info!("🔄 开始收集所有用户余额数据...");
        let collection_result = self.ranking_service.collect_all_balances().await?;
        info!("✅ 余额收集完成 - 结果: {}", collection_result.message);
        info!(
            "📊 收集统计: 成功 {} 个, 失败 {} 个",
            collection_result.collected_count, collection_result.failed_count
        );

        if collection_result.collected_count == 0 {
            info!("没有成功收集到任何余额数据，跳过排名更新");
            return Ok(());
        }

        // 2. 更新排名表
        info!("🔄 开始更新排名表...");
        self.ranking_service.update_fixed_rankings_public().await?;
        info!("✅ 排名表更新完成");

        // 3. 检查是否有新的翻仓用户
        if let Err(e) = self.check_and_congratulate_doubled_users().await {
            warn!("检查翻仓用户失败: {}", e);
        }

        info!("✅ 每小时排名更新完成");
        Ok(())
    }

    // 每日晚上8点Top5播报执行
    async fn execute_daily_top5_broadcast(&self) -> Result<()> {
        info!("📢 开始执行每日晚上8点Top5播报...");

        // 1. 确保排名数据是最新的
        info!("🔄 开始收集所有用户余额数据...");
        let collection_result = self.ranking_service.collect_all_balances().await?;
        info!("✅ 余额收集完成 - 结果: {}", collection_result.message);

        if collection_result.collected_count == 0 {
            info!("没有成功收集到任何余额数据，跳过Top5播报");
            return Ok(());
        }

        // 2. 更新排名表
        info!("🔄 开始更新排名表...");
        self.ranking_service.update_fixed_rankings_public().await?;
        info!("✅ 排名表更新完成");

        // 3. 生成钉钉排名消息（前5名）
        info!("🔄 开始生成钉钉Top5排名消息...");
        let ranking_url = format!("{}/rankings", self.web_base_url);
        info!("🔗 排名页面URL: {}", ranking_url);
        let ranking_message = self
            .ranking_service
            .get_dingtalk_ranking_message(&ranking_url)
            .await?;
        info!(
            "✅ Top5排名消息生成成功 - 前{}名用户, 总参赛人数: {}",
            ranking_message.top_rankings.len(),
            ranking_message.total_participants
        );

        // 4. 发送钉钉Top5排名通知
        info!("📤 开始发送钉钉Top5排名通知...");
        if let Err(e) = self.send_top5_ranking_notification(&ranking_message).await {
            error!("❌ 发送钉钉Top5排名通知失败: {}", e);
        } else {
            info!("✅ 每日Top5排名播报发送成功");
        }

        Ok(())
    }

    // 检查并祝贺翻仓用户
    async fn check_and_congratulate_doubled_users(&self) -> Result<()> {
        info!("🎉 开始检查翻仓用户...");

        // 获取最新的排名数据
        let rankings = self.ranking_service.get_rankings().await?;
        let daily_rankings = rankings.daily_rankings;

        if daily_rankings.is_empty() {
            info!("没有排名数据，跳过翻仓检查");
            return Ok(());
        }

        let mut congratulated_users = self.congratulated_users.lock().await;

        for entry in daily_rankings {
            if entry.is_doubled {
                let user_key = format!("{}_{}", entry.user_id, entry.exchange_type);

                // 检查是否已经祝贺过这个用户
                if !congratulated_users.contains(&user_key) {
                    info!(
                        "🎉 发现新翻仓用户: {} ({})",
                        entry.user_name, entry.exchange_type
                    );

                    // 发送祝贺消息
                    if let Err(e) = self.send_congratulation_message(&entry).await {
                        error!("发送翻仓祝贺消息失败: {}", e);
                    } else {
                        // 记录已祝贺的用户
                        congratulated_users.insert(user_key);
                        info!("✅ 翻仓祝贺消息发送成功: {}", entry.user_name);
                    }
                }
            }
        }

        Ok(())
    }

    // 获取下次每日播报时间（晚上8点）
    fn get_next_daily_broadcast_time(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        // 转换当前UTC时间到中国时间
        let now_china = TimezoneUtil::utc_to_china(now);

        // 设置为中国时间每日20点执行
        let china_tz = TimezoneUtil::china_timezone();
        let target_time_china = now_china.date_naive().and_hms_opt(20, 0, 0).unwrap();
        let target_datetime_china = china_tz.from_local_datetime(&target_time_china).unwrap();

        // 转换回UTC时间
        let target_datetime_utc = TimezoneUtil::china_to_utc(target_datetime_china);

        if now >= target_datetime_utc {
            // 如果今天中国时间20点已经过了，设置为明天20点
            target_datetime_utc + Duration::days(1)
        } else {
            target_datetime_utc
        }
    }

    // 发送Top5排名通知
    async fn send_top5_ranking_notification(
        &self,
        ranking_message: &crate::models::ranking::DingTalkRankingMessage,
    ) -> Result<()> {
        let mut message_content = String::new();
        message_content.push_str("🏆 每日交易大赛排名榜 (晚上8点播报)\n\n");
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

            let change_icon = if entry.change_amount >= Decimal::ZERO {
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

        self.dingtalk_bot
            .send_text_message(&message_content)
            .await?;
        Ok(())
    }

    // 发送翻仓祝贺消息
    async fn send_congratulation_message(
        &self,
        entry: &crate::models::ranking::RankingEntry,
    ) -> Result<()> {
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

        message_content.push_str("🏆 查看详细排名:\n");
        message_content.push_str(&format!("🔗 {}/rankings\n\n", self.web_base_url));
        message_content.push_str("💪 恭喜你实现翻仓目标！继续保持，再创佳绩！");

        self.dingtalk_bot
            .send_text_message(&message_content)
            .await?;
        Ok(())
    }

    // 原有的每日任务（保留兼容性）
    async fn execute_daily_task(&self) -> Result<()> {
        info!("开始执行每日余额收集和排名任务...");

        // 1. 收集所有用户的余额数据
        info!("🔄 开始收集所有用户余额数据...");
        let collection_result = self.ranking_service.collect_all_balances().await?;
        info!("✅ 余额收集完成 - 结果: {}", collection_result.message);
        info!(
            "📊 收集统计: 成功 {} 个, 失败 {} 个",
            collection_result.collected_count, collection_result.failed_count
        );

        if collection_result.collected_count == 0 {
            info!("没有成功收集到任何余额数据，跳过排名计算");
            return Ok(());
        }

        // 2. 等待一小段时间确保数据库事务完成
        info!("⏳ 等待数据库事务完成...");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // 3. 生成钉钉排名消息（前5名）
        info!("🔄 开始生成钉钉排名消息...");
        let ranking_url = format!("{}/rankings", self.web_base_url);
        info!("🔗 排名页面URL: {}", ranking_url);
        let ranking_message = self
            .ranking_service
            .get_dingtalk_ranking_message(&ranking_url)
            .await?;
        info!(
            "✅ 排名消息生成成功 - 前{}名用户, 总参赛人数: {}",
            ranking_message.top_rankings.len(),
            ranking_message.total_participants
        );

        // 4. 发送钉钉排名通知
        info!("📤 开始发送钉钉排名通知...");
        if let Err(e) = self.send_ranking_notification(&ranking_message).await {
            error!("❌ 发送钉钉排名通知失败: {}", e);
        } else {
            info!("✅ 每日排名播报发送成功");
        }

        Ok(())
    }

    fn get_next_run_time(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        // 转换当前UTC时间到中国时间
        let now_china = TimezoneUtil::utc_to_china(now);

        // 设置为中国时间每日8点执行
        let china_tz = TimezoneUtil::china_timezone();
        let target_time_china = now_china.date_naive().and_hms_opt(8, 0, 30).unwrap();
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

    async fn send_ranking_notification(
        &self,
        ranking_message: &crate::models::ranking::DingTalkRankingMessage,
    ) -> Result<()> {
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

            let change_icon = if entry.change_amount >= Decimal::ZERO {
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

        self.dingtalk_bot
            .send_text_message(&message_content)
            .await?;
        Ok(())
    }

    // 手动触发余额查询（用于测试）
    pub async fn trigger_balance_query(&self) -> Result<()> {
        info!("手动触发余额查询...");
        self.execute_daily_task().await
    }
}
