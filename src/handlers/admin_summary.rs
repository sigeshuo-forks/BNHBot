use crate::models::registration::RegistrationStatus;
use crate::services::{AuthService, ExchangeService, RegistrationService};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct BalanceInfo {
    pub asset: String,
    pub free: Decimal,
    pub locked: Decimal,
    pub total: Decimal,
}

#[derive(Debug, Serialize)]
pub struct AccountSummaryResponse {
    pub success: bool,
    pub message: String,
    pub query_time: DateTime<Utc>,
    pub total_usdt_value: Option<Decimal>,
    pub balances: Option<Vec<BalanceInfo>>,
}

/// 获取注册用户的账户总览（包含USDT总估值）
pub async fn get_registration_summary(
    State((registration_service, _auth_service, exchange_service)): State<(
        RegistrationService,
        AuthService,
        ExchangeService,
    )>,
    Path(registration_id): Path<String>,
) -> Result<Json<AccountSummaryResponse>, StatusCode> {
    // 解析UUID
    let id = Uuid::parse_str(&registration_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // 获取注册信息
    let registration = registration_service
        .get_registration_by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // 检查状态是否为已通过
    if registration.status != RegistrationStatus::Approved {
        return Ok(Json(AccountSummaryResponse {
            success: false,
            message: "只能查询已通过审核的用户账户总览".to_string(),
            query_time: Utc::now(),
            total_usdt_value: None,
            balances: None,
        }));
    }

    // 构建UserExchange
    let user_exchange = crate::models::UserExchange {
        id: Uuid::new_v4(), // 临时ID
        user_id: registration.id,
        exchange_type: match registration.exchange {
            crate::models::registration::RegistrationExchangeType::Binance => {
                crate::models::ExchangeType::Binance
            }
            crate::models::registration::RegistrationExchangeType::OKX => {
                crate::models::ExchangeType::Okx
            }
            crate::models::registration::RegistrationExchangeType::WEEX => {
                crate::models::ExchangeType::Weex
            }
        },
        api_key: registration.api_key,
        secret_key: registration.secret_key,
        passphrase: registration.passphrase,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        is_active: true,
    };

    // 查询账户总览
    match exchange_service.get_account_summary(&user_exchange).await {
        Ok(account_summary) => {
            let balances: Vec<BalanceInfo> = account_summary
                .balances
                .into_iter()
                .map(|eb| BalanceInfo {
                    asset: eb.asset,
                    free: eb.free,
                    locked: eb.locked,
                    total: eb.total,
                })
                .collect();

            Ok(Json(AccountSummaryResponse {
                success: true,
                message: format!("成功获取{}账户总览", registration.user_name),
                query_time: Utc::now(),
                total_usdt_value: Some(account_summary.total_usdt_value),
                balances: Some(balances),
            }))
        }
        Err(e) => {
            log::error!("查询交易所账户总览失败: {}", e);
            Ok(Json(AccountSummaryResponse {
                success: false,
                message: format!("查询交易所账户总览失败: {}", e),
                query_time: Utc::now(),
                total_usdt_value: None,
                balances: None,
            }))
        }
    }
}
