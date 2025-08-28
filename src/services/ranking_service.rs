use crate::models::ranking::*;
use crate::services::{DatabaseService, ExchangeService};
use crate::models::{UserExchange, ExchangeType};
use anyhow::Result;
use chrono::Utc;
use rust_decimal::Decimal;
use uuid::Uuid;
use std::collections::HashMap;
use log::{info, error, warn};

#[derive(Clone)]
pub struct RankingService {
    database: DatabaseService,
    exchange_service: ExchangeService,
}

impl RankingService {
    pub fn new(database: DatabaseService, exchange_service: ExchangeService) -> Self {
        Self {
            database,
            exchange_service,
        }
    }

    /// 收集所有已批准用户的余额数据
    pub async fn collect_all_balances(&self) -> Result<BalanceCollectionResponse> {
        info!("开始收集所有用户的余额数据...");
        
        let users = self.database.get_all_approved_users_with_exchanges().await?;
        let mut collected_count = 0;
        let mut failed_count = 0;
        let mut errors = Vec::new();
        
        let today = Utc::now().format("%Y-%m-%d").to_string();
        
        for (user_id, user_name, exchange_type) in users {
            match self.collect_user_balance(user_id, &user_name, &exchange_type, &today).await {
                Ok(_) => {
                    collected_count += 1;
                    info!("✅ 成功收集用户 {} ({}) 的余额数据", user_name, exchange_type);
                }
                Err(e) => {
                    failed_count += 1;
                    let error_msg = format!("用户 {} ({}): {}", user_name, exchange_type, e);
                    errors.push(error_msg.clone());
                    error!("❌ 收集用户余额失败: {}", error_msg);
                }
            }
        }
        
        info!("余额收集完成 - 成功: {}, 失败: {}", collected_count, failed_count);
        
        // 如果有成功收集的数据，重新计算并保存排名
        if collected_count > 0 {
            info!("开始计算并更新固定排名表...");
            if let Err(e) = self.update_fixed_rankings(&today).await {
                error!("更新固定排名表失败: {}", e);
                errors.push(format!("更新排名表失败: {}", e));
            } else {
                info!("✅ 固定排名表更新成功");
            }
        }
        
        Ok(BalanceCollectionResponse {
            success: failed_count == 0,
            message: format!("收集完成 - 成功: {}, 失败: {}", collected_count, failed_count),
            collected_count,
            failed_count,
            errors,
        })
    }

    /// 收集单个用户的余额数据
    async fn collect_user_balance(
        &self,
        user_id: Uuid,
        user_name: &str,
        exchange_type: &str,
        recorded_date: &str,
    ) -> Result<()> {
        // 从数据库获取用户的API配置
        let registration = self.database.get_registration_by_user_and_exchange(user_name, exchange_type).await?;
        
        // 直接使用Registration中已经解析好的交易所类型，避免重复解析
        let user_exchange = UserExchange {
            id: user_id,
            user_id,
            exchange_type: match registration.exchange {
                crate::models::registration::RegistrationExchangeType::Binance => ExchangeType::Binance,
                crate::models::registration::RegistrationExchangeType::OKX => ExchangeType::Okx,
                crate::models::registration::RegistrationExchangeType::WEEX => ExchangeType::Weex,
            },
            api_key: registration.api_key,
            secret_key: registration.secret_key,
            passphrase: registration.passphrase,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_active: true,
        };

        // 获取账户总览
        let summary = self.exchange_service.get_account_summary(&user_exchange).await?;
        
        // 将余额详情序列化为JSON
        let balance_details = serde_json::to_string(&summary.balances)?;
        
        // 保存到数据库
        self.database.save_balance_history(
            user_id,
            user_name,
            exchange_type,
            summary.total_usdt_value,
            &balance_details,
            recorded_date,
        ).await?;

        Ok(())
    }

    /// 计算排名
    pub async fn calculate_rankings(&self, period: RankingPeriod) -> Result<Vec<RankingEntry>> {
        let days = match period {
            RankingPeriod::Daily => 1,
            RankingPeriod::Weekly => 7,
            RankingPeriod::Monthly => 30,
        };

        let history = self.database.get_latest_balance_history(days).await?;
        let mut user_data: HashMap<String, Vec<BalanceHistory>> = HashMap::new();

        // 按用户分组历史数据
        for record in history {
            let key = format!("{}_{}", record.user_name, record.exchange_type);
            user_data.entry(key).or_insert_with(Vec::new).push(record);
        }

        let mut rankings = Vec::new();

        for (_, mut user_history) in user_data {
            if user_history.is_empty() {
                continue;
            }

            // 按日期排序（最新的在前）
            user_history.sort_by(|a, b| b.recorded_date.cmp(&a.recorded_date));

            let current_balance = user_history[0].total_usdt_value;
            let previous_balance = if user_history.len() > 1 {
                Some(user_history[user_history.len() - 1].total_usdt_value)
            } else {
                None
            };

            let change_amount = if let Some(prev) = previous_balance {
                current_balance - prev
            } else {
                Decimal::ZERO
            };

            let change_percentage = if let Some(prev) = previous_balance {
                if prev > Decimal::ZERO {
                    (change_amount / prev) * Decimal::from(100)
                } else {
                    Decimal::ZERO
                }
            } else {
                Decimal::ZERO
            };

            // 检查是否翻倍
            let is_doubled = if let Some(prev) = previous_balance {
                current_balance >= prev * Decimal::from(2)
            } else {
                false
            };

            // 构建历史点数据
            let balance_history: Vec<BalanceHistoryPoint> = user_history
                .iter()
                .rev() // 反转以获得时间顺序
                .map(|h| BalanceHistoryPoint {
                    date: h.recorded_date.clone(),
                    balance: h.total_usdt_value,
                })
                .collect();

            let user_label = Self::generate_user_label(
                current_balance,
                change_percentage,
                is_doubled,
                &user_history
            );

            // 获取用户身份信息
            let identity = match self.database.get_registration_by_user_and_exchange(
                &user_history[0].user_name, 
                &user_history[0].exchange_type
            ).await {
                Ok(registration) => registration.identity.to_string(),
                Err(_) => "Regular".to_string(), // 默认为普通用户
            };

            // 计算参与天数（有余额记录的天数）
            let participation_days = user_history.len() as u32;

            rankings.push(RankingEntry {
                user_id: user_history[0].user_id,
                user_name: user_history[0].user_name.clone(),
                exchange_type: user_history[0].exchange_type.clone(),
                current_balance,
                previous_balance,
                change_amount,
                change_percentage,
                rank: 0, // 稍后设置
                is_doubled,
                balance_history,
                user_label,
                identity,
                participation_days,
            });
        }

        // 按增长比例排序（从高到低）
        rankings.sort_by(|a, b| b.change_percentage.cmp(&a.change_percentage));

        // 设置排名
        for (index, entry) in rankings.iter_mut().enumerate() {
            entry.rank = (index + 1) as u32;
        }

        Ok(rankings)
    }

    /// 生成用户标签
    fn generate_user_label(
        current_balance: Decimal,
        change_percentage: Decimal,
        is_doubled: bool,
        user_history: &[BalanceHistory],
    ) -> String {

        // 爆仓 (收益率 < 0 且 账户余额为0)
        if change_percentage < Decimal::from(0) && current_balance <= Decimal::from(0) {
            return "💥 重头再来".to_string();
        }

        // 币圈新手 (历史记录少)
        if user_history.len() <= 3 && change_percentage == Decimal::from(0) {
            return "🌱 币圈新手".to_string();
        }

        // 翻倍达人
        if is_doubled {
            return "🚀 翻倍达人".to_string();
        }

        // 交易大神 (收益率 > 50%)
        if change_percentage >= Decimal::from(50) {
            return "🔥 先天圣体".to_string();
        }

        // 量化高手 (收益率 > 20% 且收益率 < 50%)
        if change_percentage >= Decimal::from(20) && change_percentage < Decimal::from(50) {
            return "🤖 交易大神".to_string();
        }

        // 稳健投资者 (收益率 5%-20%)
        if change_percentage >= Decimal::from(10) && change_percentage < Decimal::from(20) {
            return "💎 稳健投资".to_string();
        }

        // 佛系持币 (收益率 -5% 到 5%)
        if change_percentage >= Decimal::from(5) && change_percentage < Decimal::from(10) {
            return "🧘 佛系持币".to_string();
        }

        // 追涨杀跌王 (收益率 < -5%)
        if change_percentage < Decimal::from(-5) {
            return "📉 抄顶逃底".to_string();
        }

        // 默认标签
        "🛁 摸鱼高手".to_string()
    }

    /// 更新固定排名表
    async fn update_fixed_rankings(&self, recorded_date: &str) -> Result<()> {
        info!("开始更新固定排名表 - 日期: {}", recorded_date);

        // 计算三个周期的排名
        let daily_rankings = self.calculate_rankings(RankingPeriod::Daily).await?;
        let weekly_rankings = self.calculate_rankings(RankingPeriod::Weekly).await?;
        let monthly_rankings = self.calculate_rankings(RankingPeriod::Monthly).await?;

        // 保存到数据库
        self.database.save_rankings(&daily_rankings, "daily", recorded_date).await?;
        self.database.save_rankings(&weekly_rankings, "weekly", recorded_date).await?;
        self.database.save_rankings(&monthly_rankings, "monthly", recorded_date).await?;

        // 清理旧数据
        self.database.cleanup_old_rankings().await?;

        info!("固定排名表更新完成 - 日榜: {} 位, 周榜: {} 位, 月榜: {} 位", 
              daily_rankings.len(), weekly_rankings.len(), monthly_rankings.len());

        Ok(())
    }

    /// 获取固定排名响应（从数据库读取）
    pub async fn get_rankings(&self) -> Result<RankingResponse> {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        
        // 尝试从数据库获取今天的排名
        let daily_rankings = self.get_fixed_rankings("daily", &today).await?;
        let weekly_rankings = self.get_fixed_rankings("weekly", &today).await?;
        let monthly_rankings = self.get_fixed_rankings("monthly", &today).await?;

        Ok(RankingResponse {
            daily_rankings,
            weekly_rankings,
            monthly_rankings,
            last_updated: Utc::now(),
        })
    }

    /// 获取固定排名数据，如果没有则降级到实时计算
    pub async fn get_fixed_rankings(&self, period: &str, recorded_date: &str) -> Result<Vec<RankingEntry>> {
        // 首先尝试从数据库获取固定排名
        match self.database.get_rankings(period, recorded_date).await {
            Ok(rankings) if !rankings.is_empty() => {
                info!("从数据库获取{}排名 - {} 位用户", period, rankings.len());
                return Ok(rankings);
            }
            _ => {
                // 如果没有固定排名，尝试获取最近的排名
                if let Ok(Some(latest_date)) = self.database.get_latest_ranking_date(period).await {
                    if let Ok(rankings) = self.database.get_rankings(period, &latest_date).await {
                        if !rankings.is_empty() {
                            info!("使用最近的{}排名数据 - 日期: {}, {} 位用户", period, latest_date, rankings.len());
                            return Ok(rankings);
                        }
                    }
                }
                
                // 如果都没有，降级到实时计算
                warn!("未找到固定{}排名，降级到实时计算", period);
                let ranking_period = match period {
                    "daily" => RankingPeriod::Daily,
                    "weekly" => RankingPeriod::Weekly,
                    "monthly" => RankingPeriod::Monthly,
                    _ => RankingPeriod::Daily,
                };
                self.calculate_rankings(ranking_period).await
            }
        }
    }

    /// 获取钉钉排名消息（前5名）
    pub async fn get_dingtalk_ranking_message(&self, ranking_url: &str) -> Result<DingTalkRankingMessage> {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        info!("🔍 获取钉钉排名消息 - 日期: {}", today);
        
        let daily_rankings = self.get_fixed_rankings("daily", &today).await?;
        info!("📊 获取到日榜排名数据: {} 位用户", daily_rankings.len());
        
        let top_rankings: Vec<RankingEntry> = daily_rankings.into_iter().take(5).collect();
        info!("🏆 提取前5名排名数据: {} 位用户", top_rankings.len());
        
        let total_participants = self.database.get_all_approved_users_with_exchanges().await?.len() as u32;
        info!("👥 总参赛人数: {} 人", total_participants);

        Ok(DingTalkRankingMessage {
            top_rankings,
            total_participants,
            ranking_url: ranking_url.to_string(),
        })
    }
}


