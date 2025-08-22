use anyhow::Result;
use clap::Parser;
use dotenv::dotenv;
use log::{info, error};
use std::env;

use bnhbot::*;

use handlers::command::{Cli, CommandHandler};
use services::{DatabaseService, ExchangeService, DingTalkBot, Scheduler};

#[tokio::main]
async fn main() -> Result<()> {
    // 加载环境变量
    dotenv().ok();
    
    // 初始化日志
    env_logger::init();
    
    info!("启动 BNHBot - 钉钉机器人交易所余额播报系统");
    
    // 获取配置
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/bnhbot.db".to_string());
    let dingtalk_webhook = env::var("DINGTALK_WEBHOOK").expect("DINGTALK_WEBHOOK 环境变量未设置");
    let dingtalk_secret = env::var("DINGTALK_SECRET").ok();
    
    // 初始化服务
    let database = DatabaseService::new(&database_url).await?;
    let exchange_service = ExchangeService::new();
    let dingtalk_bot = DingTalkBot::new(dingtalk_webhook, dingtalk_secret);
    
    // 解析命令行参数
    let cli = Cli::parse();
    
    // 创建命令处理器
    let command_handler = CommandHandler::new(
        database.clone(),
        exchange_service.clone(),
        dingtalk_bot.clone(),
    );
    
    // 处理命令
    match command_handler.handle(cli).await {
        Ok(_) => {
            info!("命令执行成功");
        }
        Err(e) => {
            error!("命令执行失败: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}

// 启动定时任务调度器的函数
async fn start_scheduler() -> Result<()> {
    info!("启动定时任务调度器...");
    
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/bnhbot.db".to_string());
    let dingtalk_webhook = env::var("DINGTALK_WEBHOOK").expect("DINGTALK_WEBHOOK 环境变量未设置");
    let dingtalk_secret = env::var("DINGTALK_SECRET").ok();
    
    let database = DatabaseService::new(&database_url).await?;
    let exchange_service = ExchangeService::new();
    let dingtalk_bot = DingTalkBot::new(dingtalk_webhook, dingtalk_secret);
    
    let scheduler = Scheduler::new(database, exchange_service, dingtalk_bot);
    scheduler.start().await?;
    
    Ok(())
}
