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
use services::{DatabaseService, ExchangeService, DingTalkBot, Scheduler, RegistrationService, AuthService};
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
        
        // 发送启动通知到群
        info!("📢 发送启动通知到钉钉群...");
        let at_all = env::var("DINGTALK_AT_ALL").unwrap_or_else(|_| "false".to_string()).parse().unwrap_or(false);
        if let Err(e) = dingtalk_bot.send_startup_notification(at_all, &config.web_base_url).await {
            warn!("⚠️  发送启动通知失败: {}", e);
        } else {
            info!("✅ 启动通知发送成功");
        }
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
    _exchange_service: ExchangeService,
    dingtalk_bot: DingTalkBot,
    registration_service: RegistrationService,
    auth_service: AuthService,
    config: AppConfig,
) -> Result<()> {
            let webhook_handler = DingTalkWebhookHandler::new(database, dingtalk_bot.clone(), config.web_base_url.clone());
    
            // 公开路由（无需认证）
        let public_routes = Router::new()
            .route("/", get(serve_home_page))
            .route("/register", get(serve_registration_form))
            .route("/admin/login", get(serve_admin_login_page))
            .route("/api/register", post(handlers::registration::handle_registration))
            .route("/api/admin/login", post(handlers::admin::admin_login))
            .route("/api/dingtalk/webhook", post(move |payload| handle_dingtalk_webhook(payload, webhook_handler.clone())))
            .route("/api/dingtalk/test", get(serve_webhook_test_page))
            .with_state((registration_service.clone(), auth_service.clone()));

        // 需要认证的管理API路由（不包括页面）
        let admin_api_routes = Router::new()
            .route("/api/admin/registrations", get(handlers::admin::get_all_registrations))
            .route("/api/admin/stats", get(handlers::admin::get_registration_stats))
            .route("/api/admin/registrations/:id/review", post(handlers::admin::review_registration))
            .route("/api/admin/registrations/:id", delete(handlers::admin::delete_registration))
            .route("/api/registrations", get(list_registrations))
            .route("/api/registrations/:id/review", post(review_registration_api))
            .with_state((registration_service.clone(), auth_service.clone()))
            .layer(axum::middleware::from_fn_with_state(auth_service.clone(), admin_auth_middleware));

        // 管理页面路由（不需要服务器端认证，由前端JavaScript处理）
        let admin_page_routes = Router::new()
            .route("/admin", get(serve_admin_page))
            .with_state((registration_service, auth_service));

        let app = Router::new()
            .merge(public_routes)
            .merge(admin_api_routes)
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
    info!("Webhook测试: {}/api/dingtalk/test", config.web_base_url);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Web路由处理函数
async fn serve_home_page() -> Html<&'static str> {
    Html(r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>BNHBot - 钉钉机器人系统</title>
        <meta charset="utf-8">
        <style>
            body { font-family: Arial, sans-serif; margin: 40px; }
            .container { max-width: 800px; margin: 0 auto; }
            .btn { display: inline-block; padding: 10px 20px; margin: 10px; 
                   background: #1890ff; color: white; text-decoration: none; border-radius: 5px; }
        </style>
    </head>
    <body>
        <div class="container">
            <h1>🤖 BNHBot 钉钉机器人系统</h1>
            <p>欢迎使用 BNHBot！这是一个集成了交易所余额查询和报名系统的钉钉机器人。</p>
            
            <h2>主要功能</h2>
            <ul>
                <li>📊 每日自动播报交易所余额</li>
                <li>📝 用户报名和审核系统</li>
                <li>🏢 支持币安、欧易、WEEX三大交易所</li>
                <li>⏰ 定时任务自动执行</li>
            </ul>
            
            <h2>快速开始</h2>
            <a href="/register" class="btn">📝 用户报名</a>
            <a href="/admin" class="btn">⚙️ 管理界面</a>
            <a href="/api/dingtalk/test" class="btn">🔧 Webhook测试</a>
            
            <h2>系统状态</h2>
            <p>✅ 定时任务调度器: 运行中</p>
            <p>✅ Web服务器: 运行中</p>
            <p>✅ 数据库: 连接正常</p>
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
    
    // 启动定时任务调度器（在后台运行）
    let scheduler_database = database.clone();
    let scheduler_exchange_service = exchange_service.clone();
    let scheduler_dingtalk_bot = dingtalk_bot.clone();
    
    tokio::spawn(async move {
        let scheduler = Scheduler::new(scheduler_database, scheduler_exchange_service, scheduler_dingtalk_bot);
        if let Err(e) = scheduler.start().await {
            error!("定时任务调度器运行失败: {}", e);
        }
    });
    
    // 启动Web服务器（用于报名表单和管理界面）
    start_web_server(database, exchange_service, dingtalk_bot, registration_service, auth_service, config.clone()).await?;
    
    Ok(())
}

// 这个函数已经不再使用，定时任务调度器现在在start_full_service中启动
