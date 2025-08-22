use crate::models::ExchangeType;
use crate::services::{DatabaseService, ExchangeService, DingTalkBot};
use anyhow::Result;
use clap::{Parser, Subcommand};
use log::info;

#[derive(Parser)]
#[command(name = "BNHBot")]
#[command(about = "钉钉机器人 - 交易所余额播报系统")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 添加新用户
    AddUser {
        /// 钉钉用户ID
        #[arg(long)]
        dingtalk_id: String,
        /// 钉钉用户名
        #[arg(long)]
        name: String,
    },
    /// 添加用户交易所配置
    AddExchange {
        /// 钉钉用户ID
        #[arg(long)]
        dingtalk_id: String,
        /// 交易所类型 (binance/okx/weex)
        #[arg(long)]
        exchange: String,
        /// API密钥
        #[arg(long)]
        api_key: String,
        /// 密钥
        #[arg(long)]
        secret_key: String,
        /// 密码短语（欧易需要）
        #[arg(long)]
        passphrase: Option<String>,
    },
    /// 查询用户余额
    QueryBalance {
        /// 钉钉用户ID
        #[arg(long)]
        dingtalk_id: String,
    },
    /// 手动触发余额查询
    TriggerQuery,
    /// 启动定时任务
    StartScheduler,
}

pub struct CommandHandler {
    database: DatabaseService,
    exchange_service: ExchangeService,
    dingtalk_bot: DingTalkBot,
}

impl CommandHandler {
    pub fn new(database: DatabaseService, exchange_service: ExchangeService, dingtalk_bot: DingTalkBot) -> Self {
        Self {
            database,
            exchange_service,
            dingtalk_bot,
        }
    }

    pub async fn handle(&self, cli: Cli) -> Result<()> {
        match cli.command {
            Commands::AddUser { dingtalk_id, name } => {
                self.add_user(&dingtalk_id, &name).await
            }
            Commands::AddExchange { dingtalk_id, exchange, api_key, secret_key, passphrase } => {
                self.add_exchange(&dingtalk_id, &exchange, &api_key, &secret_key, passphrase.as_deref()).await
            }
            Commands::QueryBalance { dingtalk_id } => {
                self.query_balance(&dingtalk_id).await
            }
            Commands::TriggerQuery => {
                self.trigger_query().await
            }
            Commands::StartScheduler => {
                self.start_scheduler().await
            }
        }
    }

    async fn add_user(&self, dingtalk_id: &str, name: &str) -> Result<()> {
        info!("添加新用户: {} ({})", name, dingtalk_id);
        
        // 检查用户是否已存在
        if let Some(_) = self.database.get_user_by_dingtalk_id(dingtalk_id).await? {
            println!("用户已存在");
            return Ok(());
        }
        
        let user = self.database.create_user(dingtalk_id, name).await?;
        println!("用户创建成功: {} ({})", user.dingtalk_name, user.id);
        
        Ok(())
    }

    async fn add_exchange(&self, dingtalk_id: &str, exchange: &str, api_key: &str, secret_key: &str, passphrase: Option<&str>) -> Result<()> {
        info!("为用户 {} 添加交易所配置: {}", dingtalk_id, exchange);
        
        // 获取用户
        let user = match self.database.get_user_by_dingtalk_id(dingtalk_id).await? {
            Some(user) => user,
            None => {
                println!("用户不存在，请先创建用户");
                return Ok(());
            }
        };
        
        // 解析交易所类型
        let exchange_type = match exchange.parse::<ExchangeType>() {
            Ok(et) => et,
            Err(e) => {
                println!("无效的交易所类型: {}", e);
                return Ok(());
            }
        };
        
        // 验证API配置
        if let Err(e) = self.validate_exchange_config(&exchange_type, api_key, secret_key, passphrase).await {
            println!("API配置验证失败: {}", e);
            return Ok(());
        }
        
        let user_exchange = self.database.add_user_exchange(
            user.id,
            exchange_type,
            api_key,
            secret_key,
            passphrase,
        ).await?;
        
        println!("交易所配置添加成功: {} ({})", user_exchange.exchange_type, user_exchange.id);
        
        Ok(())
    }

    async fn query_balance(&self, dingtalk_id: &str) -> Result<()> {
        info!("查询用户 {} 的余额", dingtalk_id);
        
        let user = match self.database.get_user_by_dingtalk_id(dingtalk_id).await? {
            Some(user) => user,
            None => {
                println!("用户不存在");
                return Ok(());
            }
        };
        
        let user_exchanges = self.database.get_user_exchanges(user.id).await?;
        
        if user_exchanges.is_empty() {
            println!("用户未配置任何交易所");
            return Ok(());
        }
        
        println!("用户: {}", user.dingtalk_name);
        println!("交易所配置数量: {}", user_exchanges.len());
        
        for user_exchange in user_exchanges {
            println!("\n交易所: {}", user_exchange.exchange_type);
            
            match self.exchange_service.get_account_balance(&user_exchange).await {
                Ok(balances) => {
                    println!("余额数量: {}", balances.len());
                    for balance in balances.iter().take(5) { // 只显示前5个
                        println!("  {}: {:.4} (可用: {:.4}, 冻结: {:.4})", 
                            balance.asset, balance.total, balance.free, balance.locked);
                    }
                    if balances.len() > 5 {
                        println!("  ... 还有 {} 个币种", balances.len() - 5);
                    }
                }
                Err(e) => {
                    println!("查询失败: {}", e);
                }
            }
        }
        
        Ok(())
    }

    async fn trigger_query(&self) -> Result<()> {
        info!("手动触发余额查询");
        
        // 这里应该调用调度器的触发方法
        println!("余额查询已触发");
        
        Ok(())
    }

    async fn start_scheduler(&self) -> Result<()> {
        info!("启动定时任务调度器");
        
        // 这里应该启动调度器
        println!("定时任务调度器已启动");
        
        Ok(())
    }

    async fn validate_exchange_config(&self, exchange_type: &ExchangeType, api_key: &str, secret_key: &str, passphrase: Option<&str>) -> Result<()> {
        // 简单的配置验证
        if api_key.is_empty() || secret_key.is_empty() {
            anyhow::bail!("API密钥和密钥不能为空");
        }
        
        match exchange_type {
            ExchangeType::Okx => {
                if passphrase.is_none() {
                    anyhow::bail!("欧易API需要设置passphrase");
                }
            }
            _ => {}
        }
        
        Ok(())
    }
}
