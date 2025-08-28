use crate::models::registration::{Registration, RegistrationStatus, RegistrationStats};
use crate::services::{RegistrationService, AuthService, ExchangeService};
use crate::services::auth::{LoginRequest, LoginResponse};
use axum::{
    extract::{Path, State},
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// 审核请求
#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    pub status: String,
    pub admin_notes: Option<String>,
}

/// 审核响应
#[derive(Debug, Serialize)]
pub struct ReviewResponse {
    pub success: bool,
    pub message: String,
}

/// 删除响应
#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

/// 余额信息
#[derive(Debug, Serialize)]
pub struct BalanceInfo {
    pub asset: String,
    pub free: Decimal,
    pub locked: Decimal,
    pub total: Decimal,
}

/// 余额响应
#[derive(Debug, Serialize)]
pub struct BalanceResponse {
    pub success: bool,
    pub message: String,
    pub query_time: DateTime<Utc>,
    pub balances: Option<Vec<BalanceInfo>>,
}

#[derive(Debug, Serialize)]
pub struct AccountSummaryResponse {
    pub success: bool,
    pub message: String,
    pub query_time: DateTime<Utc>,
    pub total_usdt_value: Option<Decimal>,
    pub balances: Option<Vec<BalanceInfo>>,
}

/// 管理员登录
pub async fn admin_login(
    State((_registration_service, auth_service, _exchange_service)): State<(RegistrationService, AuthService, ExchangeService)>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    match auth_service.login(request).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            log::error!("登录处理失败: {}", e);
            Ok(Json(LoginResponse {
                success: false,
                message: "服务器内部错误".to_string(),
                token: None,
                expires_at: None,
            }))
        }
    }
}

/// 获取所有报名记录
pub async fn get_all_registrations(
    State((registration_service, _auth_service, _exchange_service, _dingtalk_bot, _database)): State<(RegistrationService, AuthService, ExchangeService, crate::services::DingTalkBot, crate::services::DatabaseService)>,
) -> Result<Json<Vec<Registration>>, StatusCode> {
    match registration_service.get_all_registrations().await {
        Ok(registrations) => Ok(Json(registrations)),
        Err(e) => {
            log::error!("获取报名列表失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取报名统计
pub async fn get_registration_stats(
    State((registration_service, _auth_service, _exchange_service, _dingtalk_bot, _database)): State<(RegistrationService, AuthService, ExchangeService, crate::services::DingTalkBot, crate::services::DatabaseService)>,
) -> Result<Json<RegistrationStats>, StatusCode> {
    match registration_service.get_registration_stats().await {
        Ok(stats) => Ok(Json(stats)),
        Err(e) => {
            log::error!("获取统计数据失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 审核报名
pub async fn review_registration(
    State((registration_service, _auth_service, exchange_service, dingtalk_bot, database)): State<(RegistrationService, AuthService, ExchangeService, crate::services::DingTalkBot, crate::services::DatabaseService)>,
    Path(registration_id): Path<String>,
    Json(request): Json<ReviewRequest>,
) -> Result<Json<ReviewResponse>, StatusCode> {
    // 解析报名ID
    let reg_id = match Uuid::parse_str(&registration_id) {
        Ok(id) => id,
        Err(_) => {
            return Ok(Json(ReviewResponse {
                success: false,
                message: "无效的报名ID".to_string(),
            }));
        }
    };

    // 解析状态
    let status = match request.status.to_lowercase().as_str() {
        "approved" => RegistrationStatus::Approved,
        "rejected" => RegistrationStatus::Rejected,
        _ => {
            return Ok(Json(ReviewResponse {
                success: false,
                message: "无效的审核状态".to_string(),
            }));
        }
    };

    // 获取报名信息（用于通知）
    let registration_info = match database.get_registration_by_id(reg_id).await {
        Ok(Some(reg)) => Some(reg),
        Ok(None) => {
            return Ok(Json(ReviewResponse {
                success: false,
                message: "报名记录不存在".to_string(),
            }));
        }
        Err(e) => {
            log::error!("获取报名信息失败: {}", e);
            return Ok(Json(ReviewResponse {
                success: false,
                message: "获取报名信息失败".to_string(),
            }));
        }
    };

    // 执行审核
    match registration_service.review_registration(reg_id, status, request.admin_notes).await {
        Ok(_) => {
            // 如果审核通过，发送钉钉通知并收集初始余额
            if status == RegistrationStatus::Approved {
                if let Some(reg) = registration_info {
                    // 发送钉钉通知
                    tokio::spawn(crate::handlers::approval_notification::send_approval_notification(
                        dingtalk_bot,
                        database.clone(),
                        reg.user_name.clone(),
                        reg.exchange.to_string(),
                    ));
                    
                    // 收集初始余额数据
                    tokio::spawn(collect_initial_balance(
                        database.clone(),
                        exchange_service.clone(),
                        reg.id,
                        reg.user_name.clone(),
                        reg.exchange.to_string(),
                    ));
                }
            }
            
            Ok(Json(ReviewResponse {
                success: true,
                message: "审核成功".to_string(),
            }))
        },
        Err(e) => {
            log::error!("审核失败: {}", e);
            Ok(Json(ReviewResponse {
                success: false,
                message: format!("审核失败: {}", e),
            }))
        }
    }
}

/// 删除报名
pub async fn delete_registration(
    State((registration_service, _auth_service, _exchange_service, _dingtalk_bot, _database)): State<(RegistrationService, AuthService, ExchangeService, crate::services::DingTalkBot, crate::services::DatabaseService)>,
    Path(registration_id): Path<String>,
) -> Result<Json<DeleteResponse>, StatusCode> {
    // 解析报名ID
    let reg_id = match Uuid::parse_str(&registration_id) {
        Ok(id) => id,
        Err(_) => {
            return Ok(Json(DeleteResponse {
                success: false,
                message: "无效的报名ID".to_string(),
            }));
        }
    };

    // 执行删除
    match registration_service.delete_registration(reg_id).await {
        Ok(_) => Ok(Json(DeleteResponse {
            success: true,
            message: "删除成功".to_string(),
        })),
        Err(e) => {
            log::error!("删除失败: {}", e);
            Ok(Json(DeleteResponse {
                success: false,
                message: format!("删除失败: {}", e),
            }))
        }
    }
}

/// 获取账户余额
pub async fn get_registration_balance(
    State((registration_service, _auth_service, exchange_service, _dingtalk_bot, _database)): State<(RegistrationService, AuthService, ExchangeService, crate::services::DingTalkBot, crate::services::DatabaseService)>,
    Path(registration_id): Path<String>,
) -> Result<Json<BalanceResponse>, StatusCode> {
    // 解析报名ID
    let reg_id = match Uuid::parse_str(&registration_id) {
        Ok(id) => id,
        Err(_) => {
            return Ok(Json(BalanceResponse {
                success: false,
                message: "无效的报名ID".to_string(),
                query_time: Utc::now(),
                balances: None,
            }));
        }
    };

    // 获取报名记录
    let registration = match registration_service.get_registration_by_id(reg_id).await {
        Ok(Some(reg)) => reg,
        Ok(None) => {
            return Ok(Json(BalanceResponse {
                success: false,
                message: "报名记录不存在".to_string(),
                query_time: Utc::now(),
                balances: None,
            }));
        }
        Err(e) => {
            log::error!("获取报名记录失败: {}", e);
            return Ok(Json(BalanceResponse {
                success: false,
                message: "获取报名记录失败".to_string(),
                query_time: Utc::now(),
                balances: None,
            }));
        }
    };

    // 检查报名状态
    if !matches!(registration.status, RegistrationStatus::Approved) {
        return Ok(Json(BalanceResponse {
            success: false,
            message: "只能查询已通过审核的用户余额".to_string(),
            query_time: Utc::now(),
            balances: None,
        }));
    }

    // 创建用户交易所配置
    let user_exchange = crate::models::UserExchange {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(), // 临时ID
        exchange_type: match registration.exchange {
            crate::models::registration::RegistrationExchangeType::Binance => crate::models::ExchangeType::Binance,
            crate::models::registration::RegistrationExchangeType::OKX => crate::models::ExchangeType::Okx,
            crate::models::registration::RegistrationExchangeType::WEEX => crate::models::ExchangeType::Weex,
        },
        api_key: registration.api_key,
        secret_key: registration.secret_key,
        passphrase: registration.passphrase,
        created_at: registration.created_at,
        updated_at: registration.updated_at,
        is_active: true,
    };

    // 查询账户余额
    match exchange_service.get_account_balance(&user_exchange).await {
        Ok(exchange_balances) => {
            let balances: Vec<BalanceInfo> = exchange_balances
                .into_iter()
                .map(|eb| BalanceInfo {
                    asset: eb.asset,
                    free: eb.free,
                    locked: eb.locked,
                    total: eb.total,
                })
                .collect();

            Ok(Json(BalanceResponse {
                success: true,
                message: format!("成功获取{}余额信息", registration.user_name),
                query_time: Utc::now(),
                balances: Some(balances),
            }))
        }
        Err(e) => {
            log::error!("查询交易所余额失败: {}", e);
            Ok(Json(BalanceResponse {
                success: false,
                message: format!("查询交易所余额失败: {}", e),
                query_time: Utc::now(),
                balances: None,
            }))
        }
    }
}

/// 测试账户余额（审核前）
pub async fn test_registration_balance(
    State((registration_service, _auth_service, exchange_service, _dingtalk_bot, _database)): State<(RegistrationService, AuthService, ExchangeService, crate::services::DingTalkBot, crate::services::DatabaseService)>,
    Path(registration_id): Path<String>,
) -> Result<Json<BalanceResponse>, StatusCode> {
    // 解析报名ID
    let reg_id = match Uuid::parse_str(&registration_id) {
        Ok(id) => id,
        Err(_) => {
            return Ok(Json(BalanceResponse {
                success: false,
                message: "无效的报名ID".to_string(),
                query_time: Utc::now(),
                balances: None,
            }));
        }
    };

    // 获取报名记录
    let registration = match registration_service.get_registration_by_id(reg_id).await {
        Ok(Some(reg)) => reg,
        Ok(None) => {
            return Ok(Json(BalanceResponse {
                success: false,
                message: "报名记录不存在".to_string(),
                query_time: Utc::now(),
                balances: None,
            }));
        }
        Err(e) => {
            log::error!("获取报名记录失败: {}", e);
            return Ok(Json(BalanceResponse {
                success: false,
                message: "获取报名记录失败".to_string(),
                query_time: Utc::now(),
                balances: None,
            }));
        }
    };

    // 测试模式：允许待审核状态的用户进行API测试
    if !matches!(registration.status, RegistrationStatus::Pending) {
        return Ok(Json(BalanceResponse {
            success: false,
            message: "只能测试待审核状态的用户API".to_string(),
            query_time: Utc::now(),
            balances: None,
        }));
    }

    // 创建用户交易所配置
    let user_exchange = crate::models::UserExchange {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(), // 临时ID
        exchange_type: match registration.exchange {
            crate::models::registration::RegistrationExchangeType::Binance => crate::models::ExchangeType::Binance,
            crate::models::registration::RegistrationExchangeType::OKX => crate::models::ExchangeType::Okx,
            crate::models::registration::RegistrationExchangeType::WEEX => crate::models::ExchangeType::Weex,
        },
        api_key: registration.api_key,
        secret_key: registration.secret_key,
        passphrase: registration.passphrase,
        created_at: registration.created_at,
        updated_at: registration.updated_at,
        is_active: true,
    };

    // 查询账户余额
    match exchange_service.get_account_balance(&user_exchange).await {
        Ok(exchange_balances) => {
            let balances: Vec<BalanceInfo> = exchange_balances
                .into_iter()
                .map(|eb| BalanceInfo {
                    asset: eb.asset,
                    free: eb.free,
                    locked: eb.locked,
                    total: eb.total,
                })
                .collect();

            Ok(Json(BalanceResponse {
                success: true,
                message: format!("✅ API测试成功！{}的{}交易所API密钥有效，可以正常获取余额数据", 
                    registration.user_name, 
                    match registration.exchange {
                        crate::models::registration::RegistrationExchangeType::Binance => "币安",
                        crate::models::registration::RegistrationExchangeType::OKX => "欧易",
                        crate::models::registration::RegistrationExchangeType::WEEX => "WEEX",
                    }
                ),
                query_time: Utc::now(),
                balances: Some(balances),
            }))
        }
        Err(e) => {
            log::error!("API测试失败: {}", e);
            Ok(Json(BalanceResponse {
                success: false,
                message: format!("❌ API测试失败: {}。请检查API密钥、Secret Key{}是否正确", 
                    e,
                    if matches!(registration.exchange, crate::models::registration::RegistrationExchangeType::OKX) {
                        "和Passphrase"
                    } else {
                        ""
                    }
                ),
                query_time: Utc::now(),
                balances: None,
            }))
        }
    }
}

/// 收集用户初始余额数据
async fn collect_initial_balance(
    database: crate::services::DatabaseService,
    exchange_service: crate::services::ExchangeService,
    user_id: Uuid,
    user_name: String,
    exchange_type: String,
) {
    log::info!("开始收集用户 {} 的初始余额数据", user_name);
    
    // 获取用户的API配置
    let registration = match database.get_registration_by_user_and_exchange(&user_name, &exchange_type).await {
        Ok(Some(reg)) => reg,
        Ok(None) => {
            log::error!("未找到用户 {} 的注册信息", user_name);
            return;
        }
        Err(e) => {
            log::error!("获取用户注册信息失败: {}", e);
            return;
        }
    };
    
    // 创建用户交易所配置
    let user_exchange = crate::models::UserExchange {
        id: user_id,
        user_id,
        exchange_type: match registration.exchange {
            crate::models::registration::RegistrationExchangeType::Binance => crate::models::ExchangeType::Binance,
            crate::models::registration::RegistrationExchangeType::OKX => crate::models::ExchangeType::Okx,
            crate::models::registration::RegistrationExchangeType::WEEX => crate::models::ExchangeType::Weex,
        },
        api_key: registration.api_key,
        secret_key: registration.secret_key,
        passphrase: registration.passphrase,
        created_at: registration.created_at,
        updated_at: registration.updated_at,
        is_active: true,
    };
    
    // 获取账户总览
    let summary = match exchange_service.get_account_summary(&user_exchange).await {
        Ok(summary) => summary,
        Err(e) => {
            log::error!("获取用户 {} 账户总览失败: {}", user_name, e);
            return;
        }
    };
    
    // 将余额详情序列化为JSON
    let balance_details = match serde_json::to_string(&summary.balances) {
        Ok(details) => details,
        Err(e) => {
            log::error!("序列化余额详情失败: {}", e);
            return;
        }
    };
    
    // 使用用户注册日期作为初始余额记录日期
    let initial_date = registration.created_at.format("%Y-%m-%d").to_string();
    
    // 保存到数据库
    if let Err(e) = database.save_balance_history(
        user_id,
        &user_name,
        &exchange_type,
        summary.total_usdt_value,
        &balance_details,
        &initial_date,
    ).await {
        log::error!("保存用户 {} 初始余额历史失败: {}", user_name, e);
        return;
    }
    
    log::info!("✅ 成功收集用户 {} 的初始余额数据: {} USDT", user_name, summary.total_usdt_value);
}
