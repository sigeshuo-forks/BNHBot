use crate::models::{DailyReport, BalanceSummary, Balance};
use crate::services::{DatabaseService, ExchangeService, DingTalkBot};
use anyhow::Result;
use chrono::{DateTime, Utc, TimeZone, Duration};
use log::{info, error, warn};
use rust_decimal::Decimal;
// 暂时注释掉未使用的导入
// use std::collections::HashMap;
use tokio::time::{sleep, Duration as TokioDuration};
use uuid::Uuid;

pub struct Scheduler {
    database: DatabaseService,
    exchange_service: ExchangeService,
    dingtalk_bot: DingTalkBot,
}

impl Scheduler {
    pub fn new(database: DatabaseService, exchange_service: ExchangeService, dingtalk_bot: DingTalkBot) -> Self {
        Self {
            database,
            exchange_service,
            dingtalk_bot,
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("启动定时任务调度器...");
        
        loop {
            let now = Utc::now();
            let next_run = self.get_next_run_time(now);
            let sleep_duration = (next_run - now).num_seconds() as u64;
            
            info!("下次执行时间: {}, 等待 {} 秒", next_run, sleep_duration);
            sleep(TokioDuration::from_secs(sleep_duration)).await;
            
            if let Err(e) = self.execute_daily_task().await {
                error!("执行每日任务失败: {}", e);
            }
        }
    }

    fn get_next_run_time(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        // 设置为每日8点执行
        let target_time = now.date_naive().and_hms_opt(8, 0, 0).unwrap();
        let target_datetime = Utc.from_utc_datetime(&target_time);
        
        if now >= target_datetime {
            // 如果今天8点已经过了，设置为明天8点
            target_datetime + Duration::days(1)
        } else {
            target_datetime
        }
    }

    async fn execute_daily_task(&self) -> Result<()> {
        info!("开始执行每日余额查询任务...");
        
        // 1. 获取所有活跃用户
        let users = self.get_all_active_users().await?;
        info!("找到 {} 个活跃用户", users.len());
        
        if users.is_empty() {
            info!("没有活跃用户，跳过本次执行");
            return Ok(());
        }
        
        // 2. 查询每个用户的余额
        let mut all_balances = Vec::new();
        let mut user_summaries = Vec::new();
        
        for user in &users {
            match self.query_user_balances(user).await {
                Ok(balances) => {
                    all_balances.extend(balances.clone());
                    
                    // 计算用户总资产价值
                    let total_usdt_value = balances.iter()
                        .filter_map(|b| b.usdt_value)
                        .sum();
                    
                    user_summaries.push(BalanceSummary {
                        user_id: user.id,
                        user_name: user.dingtalk_name.clone(),
                        exchange_type: balances.first().map(|b| b.exchange_type.clone()).unwrap_or_else(|| crate::models::ExchangeType::Binance),
                        total_usdt_value,
                        balances,
                    });
                }
                Err(e) => {
                    warn!("查询用户 {} 余额失败: {}", user.dingtalk_name, e);
                }
            }
        }
        
        // 3. 保存余额记录到数据库
        for balance in &all_balances {
            if let Err(e) = self.database.save_balance(
                balance.user_id,
                balance.exchange_type.clone(),
                &balance.asset,
                balance.free,
                balance.locked,
                balance.total,
                balance.usdt_value,
            ).await {
                error!("保存余额记录失败: {}", e);
            }
        }
        
        // 4. 生成每日报告
        let total_value: Decimal = user_summaries.iter()
            .map(|s| s.total_usdt_value)
            .sum();
        
        let report = DailyReport {
            date: Utc::now().format("%Y-%m-%d").to_string(),
            total_users: users.len(),
            total_value,
            user_summaries,
        };
        
        // 5. 发送钉钉通知
        if let Err(e) = self.dingtalk_bot.send_daily_report(&report).await {
            error!("发送钉钉通知失败: {}", e);
        } else {
            info!("每日余额播报发送成功");
        }
        
        Ok(())
    }

    async fn get_all_active_users(&self) -> Result<Vec<crate::models::User>> {
        // 这里需要实现获取所有活跃用户的逻辑
        // 暂时返回空向量，实际实现需要从数据库查询
        // TODO: 实现从数据库获取所有活跃用户的逻辑
        Ok(vec![])
    }

    async fn query_user_balances(&self, user: &crate::models::User) -> Result<Vec<Balance>> {
        let user_exchanges = self.database.get_user_exchanges(user.id).await?;
        let mut all_balances = Vec::new();
        
        for user_exchange in user_exchanges {
            match self.exchange_service.get_account_balance(&user_exchange).await {
                Ok(exchange_balances) => {
                    for exchange_balance in exchange_balances {
                        // 这里需要获取USDT价值，实际实现需要调用价格API
                        let usdt_value = self.get_usdt_value(&exchange_balance.asset, exchange_balance.total).await;
                        
                        all_balances.push(Balance {
                            id: Uuid::new_v4(),
                            user_id: user.id,
                            exchange_type: user_exchange.exchange_type.clone(),
                            asset: exchange_balance.asset,
                            free: exchange_balance.free,
                            locked: exchange_balance.locked,
                            total: exchange_balance.total,
                            usdt_value,
                            recorded_at: Utc::now(),
                            created_at: Utc::now(),
                        });
                    }
                }
                Err(e) => {
                    warn!("查询交易所 {} 余额失败: {}", user_exchange.exchange_type, e);
                }
            }
        }
        
        Ok(all_balances)
    }

    async fn get_usdt_value(&self, _asset: &str, _amount: Decimal) -> Option<Decimal> {
        // 这里需要实现获取USDT价格的逻辑
        // 实际实现需要调用价格API（如CoinGecko、Binance等）
        // 暂时返回None
        None
    }

    // 手动触发余额查询（用于测试）
    pub async fn trigger_balance_query(&self) -> Result<()> {
        info!("手动触发余额查询...");
        self.execute_daily_task().await
    }
}
