use anyhow::Result;
use clap::Parser;
use dotenv::dotenv;
use log::{info, error, warn};
use std::env;
use std::net::SocketAddr;
use std::fs;
use std::path::Path;
use axum::{
    routing::{get, post, delete},
    http::StatusCode,
    Json, Router,
    response::Html,
};

use tower_http::cors::CorsLayer;

use bnhbot::*;
use bnhbot::middleware::{security_headers_middleware, admin_auth_middleware};

use handlers::command::{Cli, CommandHandler};
use handlers::dingtalk_webhook::{DingTalkWebhookHandler, DingTalkMessage, DingTalkResponse};
use services::{DatabaseService, ExchangeService, DingTalkBot, Scheduler, RegistrationService, AuthService, RankingService};
use models::registration::RegistrationResponse;
use utils::dingtalk_check::DingTalkChecker;
use utils::webhook_test::WebhookTester;

// 配置结构体
#[derive(Clone)]
struct AppConfig {
    database_url: String,
    dingtalk_webhook: String,
    dingtalk_secret: Option<String>,
    dingtalk_at_all: bool,
    web_host: String,
    web_port: u16,
    web_base_url: String,
    admin_password: String,
    jwt_secret: String,
    session_timeout_hours: i64,
}

impl AppConfig {
    fn from_env() -> Result<Self> {
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/bnhbot.db".to_string());
        let dingtalk_webhook = env::var("DINGTALK_WEBHOOK").expect("DINGTALK_WEBHOOK 环境变量未设置");
        let dingtalk_secret = env::var("DINGTALK_SECRET").ok();
        let dingtalk_at_all = env::var("DINGTALK_AT_ALL").unwrap_or_else(|_| "false".to_string()).parse().unwrap_or(false);
        
        let web_host = env::var("WEB_SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let web_port = env::var("WEB_SERVER_PORT").unwrap_or_else(|_| "3000".to_string()).parse().unwrap_or(3000);
        let web_base_url = env::var("WEB_SERVER_BASE_URL").unwrap_or_else(|_| format!("http://{}:{}", web_host, web_port));
        let admin_password = env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin123".to_string());
        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "default_jwt_secret_change_in_production".to_string());
        let session_timeout_hours = env::var("SESSION_TIMEOUT_HOURS").unwrap_or_else(|_| "24".to_string()).parse().unwrap_or(24);
        
        Ok(Self {
            database_url,
            dingtalk_webhook,
            dingtalk_secret,
            dingtalk_at_all,
            web_host,
            web_port,
            web_base_url,
            admin_password,
            jwt_secret,
            session_timeout_hours,
        })
    }
}

// 创建默认环境变量文件
fn create_default_env_if_needed() -> Result<()> {
    if !std::path::Path::new(".env").exists() {
        info!("📝 创建默认 .env 文件...");
        
        let env_content = r#"# BNHBot 环境变量配置

# 钉钉机器人配置
DINGTALK_WEBHOOK=https://oapi.dingtalk.com/robot/send?access_token=YOUR_ACCESS_TOKEN
DINGTALK_SECRET=YOUR_SECRET_KEY
DINGTALK_AT_ALL=false

# 数据库配置
DATABASE_URL=sqlite:data/bnhbot.db

# Web服务器配置
WEB_SERVER_HOST=127.0.0.1
WEB_SERVER_PORT=3000
WEB_SERVER_BASE_URL=http://localhost:3000

# 日志级别
RUST_LOG=info
"#;
        
        fs::write(".env", env_content).map_err(|e| {
            anyhow::anyhow!("无法创建 .env 文件: {}", e)
        })?;
        
        info!("⚠️  请编辑 .env 文件，配置钉钉机器人信息");
        info!("   1. 在钉钉群中添加自定义机器人");
        info!("   2. 获取 Webhook URL 和 Secret");
        info!("   3. 更新 .env 文件中的配置");
        info!("   4. 重新启动服务");
        
        anyhow::bail!("请先配置钉钉机器人信息，然后重新启动服务");
    }
    
    Ok(())
}

// 检查环境变量
fn check_environment_variables() -> Result<()> {
    // 检查钉钉Webhook
    if env::var("DINGTALK_WEBHOOK").is_err() {
        anyhow::bail!("DINGTALK_WEBHOOK 环境变量未设置，请检查 .env 文件");
    }
    
    // 检查数据库URL
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/bnhbot.db".to_string());
    info!("📊 数据库连接: {}", database_url);
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // 加载环境变量
    dotenv().ok();
    
    // 初始化日志
    env_logger::init();
    
    info!("启动 BNHBot - 钉钉机器人交易所余额播报系统");
    
    // 检查并创建默认环境变量文件
    create_default_env_if_needed()?;
    
    // 检查环境变量
    check_environment_variables()?;
    
    // 获取配置
    let config = AppConfig::from_env()?;
    let dingtalk_webhook_clone = config.dingtalk_webhook.clone();
    let dingtalk_secret_clone = config.dingtalk_secret.clone();
    
    // 确保数据目录存在
    info!("📁 创建数据目录...");
    std::fs::create_dir_all("data").map_err(|e| {
        anyhow::anyhow!("无法创建数据目录: {}", e)
    })?;
    
    // 检查数据库文件路径
    let db_path = if config.database_url.starts_with("sqlite:") {
        let path = config.database_url.trim_start_matches("sqlite:");
        if !path.starts_with('/') && !path.starts_with("data/") {
            format!("data/{}", path)
        } else {
            path.to_string()
        }
    } else {
        config.database_url.clone()
    };
    
    info!("🗄️  数据库文件路径: {}", db_path);
    
    // 确保数据库文件的父目录存在
    if let Some(parent) = Path::new(&db_path).parent() {
        if !parent.exists() {
            info!("📁 创建数据库父目录: {:?}", parent);
            fs::create_dir_all(parent).map_err(|e| {
                anyhow::anyhow!("无法创建数据库父目录 {:?}: {}", parent, e)
            })?;
        }
    }
    
    // 如果数据库文件不存在，尝试创建一个空的数据库文件
    if !Path::new(&db_path).exists() {
        info!("📝 创建数据库文件...");
        // 创建一个空的数据库文件
        fs::File::create(&db_path).map_err(|e| {
            anyhow::anyhow!("无法创建数据库文件 {}: {}", db_path, e)
        })?;
        info!("✅ 数据库文件创建成功");
    }
    
    // 初始化服务
    info!("🔧 初始化数据库服务...");
    let database = DatabaseService::new(&config.database_url).await?;
    info!("✅ 数据库服务初始化成功");
    let exchange_service = ExchangeService::new();
    let dingtalk_bot = DingTalkBot::new(config.dingtalk_webhook.clone(), config.dingtalk_secret.clone());
    let auth_service = AuthService::new()?;
    
    // 检查钉钉机器人配置
    info!("🔧 检查钉钉机器人配置...");
            let webhook_handler = DingTalkWebhookHandler::new(database.clone(), dingtalk_bot.clone(), config.web_base_url.clone());
    if let Err(e) = webhook_handler.check_config().await {
        warn!("⚠️  钉钉机器人配置检查失败: {}", e);
        
        // 显示详细的配置检查信息
        if let Some(secret) = &dingtalk_secret_clone {
            let check_info = DingTalkChecker::check_signature_config(&dingtalk_webhook_clone, secret);
            info!("🔍 签名配置检查详情:\n{}", check_info);
        }
        
        info!("💡 请检查钉钉机器人配置：");
        info!("   1. 确保机器人已添加到群中");
        info!("   2. 确保开启了'接收消息'权限");
        info!("   3. 确保Webhook URL正确");
        info!("   4. 确保签名密钥配置正确");
        info!("   5. 如果设置了关键词，确保消息包含关键词");
    } else {
        info!("✅ 钉钉机器人配置检查成功");
        
        // // 发送启动通知到群
        // info!("📢 发送启动通知到钉钉群...");
        // let at_all = env::var("DINGTALK_AT_ALL").unwrap_or_else(|_| "false".to_string()).parse().unwrap_or(false);
        // if let Err(e) = dingtalk_bot.send_startup_notification(at_all, &config.web_base_url).await {
        //     warn!("⚠️  发送启动通知失败: {}", e);
        // } else {
        //     info!("✅ 启动通知发送成功");
        // }
    }
    
    // 检查是否有命令行参数
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() > 1 {
        // 有命令行参数，使用命令行模式
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
    } else {
        // 无命令行参数，启动完整服务
        info!("启动 BNHBot 完整服务...");
        start_full_service(database, exchange_service, dingtalk_bot, auth_service, config).await?;
    }
    
    Ok(())
}

// 启动Web服务器
async fn start_web_server(
    database: DatabaseService,
    exchange_service: ExchangeService,
    dingtalk_bot: DingTalkBot,
    registration_service: RegistrationService,
    auth_service: AuthService,
    ranking_service: RankingService,
    config: AppConfig,
) -> Result<()> {
    // 创建调度器实例用于管理API
    let _scheduler = Scheduler::new(
        database.clone(),
        exchange_service.clone(),
        dingtalk_bot.clone(),
        ranking_service.clone(),
        config.web_base_url.clone(),
    );
            let webhook_handler = DingTalkWebhookHandler::new(database.clone(), dingtalk_bot.clone(), config.web_base_url.clone());
    
            // 基础公开路由
        let basic_routes = Router::new()
            .route("/", get(serve_home_page))
            .route("/register", get(serve_registration_form))
            .route("/rankings", get(serve_ranking_page))
            .route("/admin/login", get(serve_admin_login_page))
            .route("/api/register", post(handlers::registration::handle_registration))
            .route("/api/check-username", get(handlers::username_check::check_username_availability).with_state(database.clone()))
            .route("/api/mock-mode", get(handlers::mock_mode::get_mock_mode_public))

            .route("/api/dingtalk/webhook", post(move |payload| handle_dingtalk_webhook(payload, webhook_handler.clone())))
            .route("/api/dingtalk/test", get(serve_webhook_test_page))
            .with_state((registration_service.clone(), auth_service.clone(), exchange_service.clone()));

        // 测试余额API路由（需要ExchangeService）
        let exchange_service_for_test = exchange_service.clone();
        let test_balance_routes = Router::new()
            .route("/api/test-balance", post(move |payload| handle_test_balance(payload, exchange_service_for_test.clone())))
            .with_state(());

        // 排名相关的公开路由
        let ranking_routes = Router::new()
            .route("/api/rankings", get(handlers::ranking::get_rankings))
            .route("/api/rankings/period", get(handlers::ranking::get_period_rankings))
            .with_state((registration_service.clone(), auth_service.clone(), exchange_service.clone(), ranking_service.clone()));

        // Mock数据路由（用于演示）
        let mock_routes = Router::new()
            .route("/api/mock/rankings", get(handlers::mock_ranking::get_mock_rankings))
            .route("/api/mock/rankings/period", get(handlers::mock_ranking::get_mock_period_rankings));

        // 需要完整状态的管理API路由（包含钉钉通知功能）
        let full_state_admin_routes = Router::new()
            .route("/api/admin/registrations", get(handlers::admin::get_all_registrations))
            .route("/api/admin/stats", get(handlers::admin::get_registration_stats))
            .route("/api/admin/registrations/:id/review", post(handlers::admin::review_registration))
            .route("/api/admin/registrations/:id", delete(handlers::admin::delete_registration))
            .route("/api/admin/registrations/:id/balance", get(handlers::admin::get_registration_balance))
            .route("/api/admin/registrations/:id/test-balance", get(handlers::admin::test_registration_balance))
            .with_state((registration_service.clone(), auth_service.clone(), exchange_service.clone(), dingtalk_bot.clone(), database.clone()))
            .layer(axum::middleware::from_fn_with_state(auth_service.clone(), admin_auth_middleware));

        // 管理员登录路由（无需认证）
        let admin_login_routes = Router::new()
            .route("/api/admin/login", post(handlers::admin::admin_login))
            .with_state((registration_service.clone(), auth_service.clone(), exchange_service.clone()));

        // 基础管理API路由（需要认证但不需要钉钉和数据库）
        let basic_admin_routes = Router::new()
            .route("/api/admin/registrations/:id/summary", get(handlers::admin_summary::get_registration_summary))
            .route("/api/admin/mock-mode", get(handlers::mock_mode::get_mock_mode))
            .route("/api/admin/mock-mode", post(handlers::mock_mode::set_mock_mode))
            .route("/api/registrations", get(list_registrations))
            .route("/api/registrations/:id/review", post(review_registration_api))
            .with_state((registration_service.clone(), auth_service.clone(), exchange_service.clone()))
            .layer(axum::middleware::from_fn_with_state(auth_service.clone(), admin_auth_middleware));

        // 排名相关的管理API路由
        let ranking_admin_routes = Router::new()
            .with_state((registration_service.clone(), auth_service.clone(), exchange_service.clone(), ranking_service.clone()))
            .layer(axum::middleware::from_fn_with_state(auth_service.clone(), admin_auth_middleware));

        // 手动发送排名的管理API路由（基于已有数据，不重新获取交易所数据）
        let manual_ranking_admin_routes = Router::new()
            .route("/api/admin/send-ranking", post(handlers::admin_ranking::send_ranking_to_dingtalk))
            .with_state((ranking_service.clone(), dingtalk_bot.clone()))
            .layer(axum::middleware::from_fn_with_state(auth_service.clone(), admin_auth_middleware));

        // 手动更新排名表的管理API路由
        let update_ranking_admin_routes = Router::new()
            .route("/api/admin/update-rankings", post(handlers::admin_ranking::update_fixed_rankings))
            .with_state(ranking_service.clone())
            .layer(axum::middleware::from_fn_with_state(auth_service.clone(), admin_auth_middleware));

        // 需要钉钉机器人的管理API路由
        let dingtalk_admin_routes = Router::new()
            .route("/api/admin/trigger-hourly-update", post(handlers::admin_ranking::trigger_hourly_update))
            .route("/api/admin/trigger-top5-broadcast", post(handlers::admin_ranking::trigger_top5_broadcast))
            .route("/api/admin/check-congratulations", post(handlers::admin_ranking::check_congratulations))
            .with_state((ranking_service.clone(), dingtalk_bot.clone()))
            .layer(axum::middleware::from_fn_with_state(auth_service.clone(), admin_auth_middleware));



        // 管理页面路由（不需要服务器端认证，由前端JavaScript处理）
        let admin_page_routes = Router::new()
            .route("/admin", get(serve_admin_page))
            .with_state((registration_service, auth_service, exchange_service));

        let app = Router::new()
            .merge(basic_routes)
            .merge(test_balance_routes)
            .merge(ranking_routes)
            .merge(mock_routes)
            .merge(admin_login_routes)
            .merge(full_state_admin_routes)
            .merge(basic_admin_routes)
            .merge(ranking_admin_routes)
            .merge(manual_ranking_admin_routes)
            .merge(update_ranking_admin_routes)
            .merge(dingtalk_admin_routes)
            .merge(admin_page_routes)
            .layer(axum::middleware::from_fn(security_headers_middleware))
            .layer(CorsLayer::permissive());

    // 解析IP地址
    let ip_parts: Vec<u8> = config.web_host.split('.')
        .map(|s| s.parse().unwrap_or(127))
        .collect();
    let ip = if ip_parts.len() == 4 {
        [ip_parts[0], ip_parts[1], ip_parts[2], ip_parts[3]]
    } else {
        [127, 0, 0, 1]
    };
    
    let addr = SocketAddr::from((ip, config.web_port));
    info!("Web服务器启动在: {}", config.web_base_url);
    info!("报名表单: {}/register", config.web_base_url);
    info!("管理界面: {}/admin", config.web_base_url);
    info!("排名页面: {}/rankings", config.web_base_url);
    info!("Webhook测试: {}/api/dingtalk/test", config.web_base_url);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Web路由处理函数
async fn serve_home_page() -> Html<&'static str> {
    Html(r#"
    <!DOCTYPE html>
    <html lang="zh-CN">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
        <title>BNHBot - 交易大赛机器人</title>
        <style>
            * {
                margin: 0;
                padding: 0;
                box-sizing: border-box;
            }

            body {
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
                background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                min-height: 100vh;
                color: #333;
            }

            /* 导航栏样式 */
            .navbar {
                background: rgba(255, 255, 255, 0.1);
                backdrop-filter: blur(10px);
                padding: 15px 0;
                box-shadow: 0 2px 20px rgba(0,0,0,0.1);
            }

            .navbar .nav-container {
                display: flex;
                justify-content: space-between;
                align-items: center;
                max-width: 1200px;
                margin: 0 auto;
                padding: 0 20px;
            }

            .navbar-brand {
                color: white;
                font-size: 24px;
                font-weight: bold;
                text-decoration: none;
                display: flex;
                align-items: center;
                gap: 10px;
            }

            .navbar-nav {
                display: flex;
                gap: 20px;
                list-style: none;
                margin: 0;
                padding: 0;
            }

            .nav-link {
                color: white;
                text-decoration: none;
                padding: 8px 16px;
                border-radius: 20px;
                transition: all 0.3s ease;
                font-weight: 500;
            }

            .nav-link:hover, .nav-link.active {
                background: rgba(255, 255, 255, 0.2);
                transform: translateY(-2px);
            }

            .container {
                max-width: 1200px;
                margin: 0 auto;
                padding: 40px 20px;
            }

            .hero {
                text-align: center;
                color: white;
                margin-bottom: 60px;
            }

            .hero h1 {
                font-size: 3.5rem;
                margin-bottom: 20px;
                text-shadow: 0 2px 10px rgba(0,0,0,0.3);
            }

            .hero p {
                font-size: 1.2rem;
                margin-bottom: 30px;
                opacity: 0.9;
            }

            .features {
                display: grid;
                grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
                gap: 30px;
                margin-bottom: 60px;
            }

            .feature-card {
                background: rgba(255, 255, 255, 0.95);
                padding: 30px;
                border-radius: 20px;
                box-shadow: 0 10px 30px rgba(0,0,0,0.1);
                transition: transform 0.3s ease;
                text-align: center;
            }

            .feature-card:hover {
                transform: translateY(-10px);
            }

            .feature-icon {
                font-size: 3rem;
                margin-bottom: 20px;
                display: block;
            }

            .feature-card h3 {
                color: #333;
                margin-bottom: 15px;
                font-size: 1.3rem;
            }

            .feature-card p {
                color: #666;
                line-height: 1.6;
            }

            .cta-section {
                text-align: center;
                background: rgba(255, 255, 255, 0.95);
                padding: 40px;
                border-radius: 20px;
                box-shadow: 0 10px 30px rgba(0,0,0,0.1);
            }

            .cta-section h2 {
                color: #333;
                margin-bottom: 20px;
                font-size: 2rem;
            }

            .cta-buttons {
                display: flex;
                gap: 20px;
                justify-content: center;
                flex-wrap: wrap;
                margin-top: 30px;
            }

            .btn {
                display: inline-flex;
                align-items: center;
                gap: 10px;
                padding: 15px 30px;
                background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                color: white;
                text-decoration: none;
                border-radius: 50px;
                font-weight: 600;
                transition: all 0.3s ease;
                box-shadow: 0 5px 15px rgba(102, 126, 234, 0.4);
            }

            .btn:hover {
                transform: translateY(-3px);
                box-shadow: 0 8px 25px rgba(102, 126, 234, 0.6);
            }

            .btn.secondary {
                background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
                box-shadow: 0 5px 15px rgba(240, 147, 251, 0.4);
            }

            .btn.secondary:hover {
                box-shadow: 0 8px 25px rgba(240, 147, 251, 0.6);
            }

            @media (max-width: 768px) {
                .navbar .nav-container {
                    flex-direction: column;
                    gap: 15px;
                }
                
                .navbar-nav {
                    flex-wrap: wrap;
                    justify-content: center;
                    gap: 10px;
                }

                .hero h1 {
                    font-size: 2.5rem;
                }

                .hero p {
                    font-size: 1.1rem;
                }

                .container {
                    padding: 30px 15px;
                }

                .features {
                    grid-template-columns: 1fr;
                    gap: 20px;
                }

                .feature-card {
                    padding: 25px;
                }

                .cta-section {
                    padding: 30px 20px;
                }

                .cta-section h2 {
                    font-size: 1.8rem;
                }

                .cta-buttons {
                    flex-direction: column;
                    align-items: center;
                }

                .btn {
                    width: 100%;
                    max-width: 300px;
                    justify-content: center;
                }
            }

            @media (max-width: 480px) {
                .hero h1 {
                    font-size: 2rem;
                }

                .hero p {
                    font-size: 1rem;
                }

                .container {
                    padding: 20px 10px;
                }

                .feature-card {
                    padding: 20px;
                }

                .feature-icon {
                    font-size: 2.5rem;
                }

                .cta-section {
                    padding: 25px 15px;
                }

                .cta-section h2 {
                    font-size: 1.5rem;
                }
            }

            /* 横屏手机优化 */
            @media (max-width: 768px) and (orientation: landscape) {
                .navbar .nav-container {
                    flex-direction: row;
                }

                .hero h1 {
                    font-size: 2.2rem;
                }

                .cta-buttons {
                    flex-direction: row;
                    flex-wrap: wrap;
                }

                .btn {
                    width: auto;
                    max-width: none;
                }
            }
        </style>
    </head>
    <body>
        <!-- 导航栏 -->
        <nav class="navbar">
            <div class="nav-container">
                <a href="/" class="navbar-brand">
                    🤖 BNHBot
                </a>
                <ul class="navbar-nav">
                    <li><a href="/" class="nav-link active">🏠 首页</a></li>
                    <li><a href="/register" class="nav-link">📝 报名参赛</a></li>
                    <li><a href="/rankings" class="nav-link">🏆 排名榜</a></li>
                </ul>
            </div>
        </nav>

        <div class="container">
            <!-- 英雄区域 -->
            <div class="hero">
                <h1>🤖 BNHBot</h1>
                <p>专业的交易大赛机器人，助力你的交易之路</p>
            </div>

            <!-- 功能特色 -->
            <div class="features">
                <div class="feature-card">
                    <span class="feature-icon">📊</span>
                    <h3>实时排名播报</h3>
                    <p>每日8点自动播报交易大赛排名，实时跟踪你的表现和进步</p>
                </div>
                <div class="feature-card">
                    <span class="feature-icon">🏢</span>
                    <h3>多交易所支持</h3>
                    <p>支持币安、欧易等主流交易所，一站式管理你的交易账户</p>
                </div>
                <div class="feature-card">
                    <span class="feature-icon">🏆</span>
                    <h3>竞技排名系统</h3>
                    <p>专业的排名算法，公平公正的比赛环境，展示你的交易实力</p>
                </div>
                <div class="feature-card">
                    <span class="feature-icon">📱</span>
                    <h3>钉钉集成</h3>
                    <p>无缝集成钉钉机器人，随时随地获取最新的排名和交易信息</p>
                </div>
            </div>

            <!-- 行动号召 -->
            <div class="cta-section">
                <h2>🚀 开始你的交易大赛之旅</h2>
                <p>加入我们的交易大赛，与全球交易者一较高下，证明你的交易实力！</p>
                <div class="cta-buttons">
                    <a href="/register" class="btn">📝 立即报名参赛</a>
                    <a href="/rankings" class="btn secondary">🏆 查看排名榜</a>
                </div>
            </div>
        </div>
    </body>
    </html>
    "#)
}

async fn serve_registration_form() -> Html<String> {
    // 读取简化的报名表单HTML文件
    let html_content = include_str!("../templates/registration_form.html");
    Html(html_content.to_string())
}

async fn serve_ranking_page() -> Html<String> {
    // 读取排名页面HTML文件
    let html_content = include_str!("../templates/ranking_page.html");
    Html(html_content.to_string())
}

async fn serve_admin_page() -> Html<String> {
    // 读取管理页面HTML文件
    let html_content = include_str!("../templates/admin_page.html");
    Html(html_content.to_string())
}

async fn serve_admin_login_page() -> Html<String> {
    // 读取管理员登录页面HTML文件
    let html_content = include_str!("../templates/admin_login.html");
    Html(html_content.to_string())
}



async fn list_registrations() -> Json<Vec<String>> {
    // 这里应该返回实际的报名列表
    Json(vec!["报名1".to_string(), "报名2".to_string()])
}

async fn review_registration_api() -> Json<RegistrationResponse> {
    // 这里应该处理审核逻辑
    Json(RegistrationResponse {
        success: true,
        message: "审核完成".to_string(),
        registration_id: None,
    })
}

// 钉钉Webhook处理函数
async fn handle_dingtalk_webhook(
    Json(payload): Json<DingTalkMessage>,
    webhook_handler: DingTalkWebhookHandler,
) -> Result<Json<DingTalkResponse>, StatusCode> {
    match webhook_handler.handle_message(payload).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            error!("处理钉钉Webhook消息失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Webhook测试页面
async fn serve_webhook_test_page() -> Html<String> {
    // 这里需要从配置中获取，暂时使用默认值
    let webhook_url = "http://localhost:3000/api/dingtalk/webhook";
    let curl_commands = WebhookTester::generate_curl_commands(webhook_url);
    
    let html_content = format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>钉钉Webhook测试</title>
            <meta charset="utf-8">
            <style>
                body {{ font-family: Arial, sans-serif; margin: 40px; }}
                .container {{ max-width: 1000px; margin: 0 auto; }}
                .test-section {{ margin: 20px 0; padding: 20px; border: 1px solid #ddd; border-radius: 5px; }}
                .curl-command {{ background: #f5f5f5; padding: 15px; border-radius: 5px; font-family: monospace; white-space: pre-wrap; }}
                .info-box {{ background: #e3f2fd; padding: 15px; border-radius: 5px; margin: 20px 0; }}
                .warning-box {{ background: #fff3e0; padding: 15px; border-radius: 5px; margin: 20px 0; }}
            </style>
        </head>
        <body>
            <div class="container">
                <h1>🔧 钉钉Webhook测试工具</h1>
                
                <div class="info-box">
                    <h3>📋 测试说明</h3>
                    <p>使用以下curl命令测试钉钉Webhook是否正常工作。如果机器人能正确回复，说明Webhook配置正确。</p>
                </div>
                
                <div class="warning-box">
                    <h3>⚠️  重要提醒</h3>
                    <p>钉钉机器人默认<strong>不能接收群消息</strong>，只能通过以下方式触发：</p>
                    <ul>
                        <li>在钉钉机器人设置中添加关键词（如"报名"）</li>
                        <li>用户发送包含关键词的消息时，钉钉会调用Webhook</li>
                        <li>或者使用钉钉开放平台API（需要企业认证）</li>
                    </ul>
                </div>
                
                <div class="test-section">
                    <h3>🧪 测试命令</h3>
                    <p>复制以下命令到终端执行：</p>
                    {}
                </div>
                
                <div class="test-section">
                    <h3>📱 钉钉机器人配置步骤</h3>
                    <ol>
                        <li>进入钉钉群 → 群设置 → 机器人管理</li>
                        <li>选择你的机器人 → 设置</li>
                        <li>在"关键词"中添加：<code>报名</code></li>
                        <li>保存设置</li>
                        <li>在群中发送包含"报名"的消息</li>
                    </ol>
                </div>
                
                <div class="test-section">
                    <h3>🔍 故障排查</h3>
                    <ul>
                        <li>检查机器人是否在群中</li>
                        <li>检查是否设置了关键词</li>
                        <li>检查Webhook URL是否正确</li>
                        <li>检查签名配置是否正确</li>
                        <li>查看服务日志输出</li>
                    </ul>
                </div>
            </div>
        </body>
        </html>
        "#,
        curl_commands.join("\n\n")
    );
    
    Html(html_content)
}

// 启动完整服务的函数
async fn start_full_service(
    database: DatabaseService,
    exchange_service: ExchangeService,
    dingtalk_bot: DingTalkBot,
    auth_service: AuthService,
    config: AppConfig,
) -> Result<()> {
    info!("启动 BNHBot 完整服务...");
    
    // 创建报名服务
    let registration_service = RegistrationService::new(database.clone());
    
    // 创建排名服务
    let ranking_service = RankingService::new(database.clone(), exchange_service.clone());
    
    // 启动定时任务调度器（在后台运行）
    let scheduler_database = database.clone();
    let scheduler_exchange_service = exchange_service.clone();
    let scheduler_dingtalk_bot = dingtalk_bot.clone();
    let scheduler_ranking_service = ranking_service.clone();
    let scheduler_web_base_url = config.web_base_url.clone();
    
    tokio::spawn(async move {
        let scheduler = Scheduler::new(scheduler_database, scheduler_exchange_service, scheduler_dingtalk_bot, scheduler_ranking_service, scheduler_web_base_url);
        if let Err(e) = scheduler.start().await {
            error!("定时任务调度器运行失败: {}", e);
        }
    });
    
    // 启动Web服务器（用于报名表单和管理界面）
    start_web_server(database, exchange_service, dingtalk_bot, registration_service, auth_service, ranking_service, config.clone()).await?;
    
    Ok(())
}

// 这个函数已经不再使用，定时任务调度器现在在start_full_service中启动

// 测试余额API处理函数
async fn handle_test_balance(
    Json(payload): Json<serde_json::Value>,
    exchange_service: ExchangeService,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // 解析请求数据
    let _user_name = payload["user_name"].as_str()
        .ok_or_else(|| StatusCode::BAD_REQUEST)?;
    let exchange = payload["exchange"].as_str()
        .ok_or_else(|| StatusCode::BAD_REQUEST)?;
    let api_key = payload["api_key"].as_str()
        .ok_or_else(|| StatusCode::BAD_REQUEST)?;
    let secret_key = payload["secret_key"].as_str()
        .ok_or_else(|| StatusCode::BAD_REQUEST)?;
    let passphrase = payload["passphrase"].as_str();

    // 验证交易所类型
    let exchange_type = match exchange {
        "binance" => crate::models::ExchangeType::Binance,
        "okx" => crate::models::ExchangeType::Okx,
        "weex" => crate::models::ExchangeType::Weex,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // 创建临时的UserExchange对象用于测试
    let test_user_exchange = crate::models::UserExchange {
        id: uuid::Uuid::new_v4(),
        user_id: uuid::Uuid::new_v4(),
        exchange_type,
        api_key: api_key.to_string(),
        secret_key: secret_key.to_string(),
        passphrase: passphrase.map(|s| s.to_string()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        is_active: true,
    };

    // 测试获取余额
    match exchange_service.get_account_summary(&test_user_exchange).await {
        Ok(summary) => {
            let response = serde_json::json!({
                "success": true,
                "message": "API测试成功",
                "total_usdt_value": summary.total_usdt_value.to_string(),
                "balances": summary.balances.iter().map(|b| {
                    serde_json::json!({
                        "asset": b.asset,
                        "free": b.free.to_string(),
                        "locked": b.locked.to_string(),
                        "total": b.total.to_string()
                    })
                }).collect::<Vec<_>>()
            });
            Ok(Json(response))
        }
        Err(e) => {
            let response = serde_json::json!({
                "success": false,
                "message": format!("API测试失败: {}", e)
            });
            Ok(Json(response))
        }
    }
}
