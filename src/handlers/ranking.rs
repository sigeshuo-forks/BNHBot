use crate::models::ranking::*;
use crate::services::RankingService;
use axum::{
    extract::{State, Query},
    response::Json,
    http::StatusCode,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RankingQuery {
    pub period: Option<String>,
    pub limit: Option<u32>,
}

/// 获取排名数据
pub async fn get_rankings(
    State((_registration_service, _auth_service, _exchange_service, ranking_service)): State<(crate::services::RegistrationService, crate::services::AuthService, crate::services::ExchangeService, RankingService)>,
    Query(_query): Query<RankingQuery>,
) -> Result<Json<RankingResponse>, StatusCode> {
    match ranking_service.get_rankings().await {
        Ok(rankings) => Ok(Json(rankings)),
        Err(e) => {
            log::error!("获取排名数据失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 手动触发余额收集（管理员功能）
pub async fn trigger_balance_collection(
    State((_registration_service, _auth_service, _exchange_service, ranking_service)): State<(crate::services::RegistrationService, crate::services::AuthService, crate::services::ExchangeService, RankingService)>,
) -> Result<Json<BalanceCollectionResponse>, StatusCode> {
    match ranking_service.collect_all_balances().await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            log::error!("触发余额收集失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取特定周期的排名
pub async fn get_period_rankings(
    State((_registration_service, _auth_service, _exchange_service, ranking_service)): State<(crate::services::RegistrationService, crate::services::AuthService, crate::services::ExchangeService, RankingService)>,
    Query(query): Query<RankingQuery>,
) -> Result<Json<Vec<RankingEntry>>, StatusCode> {
    let period = match query.period.as_deref() {
        Some("daily") => RankingPeriod::Daily,
        Some("weekly") => RankingPeriod::Weekly,
        Some("monthly") => RankingPeriod::Monthly,
        _ => RankingPeriod::Daily,
    };

    match ranking_service.calculate_rankings(period.clone()).await {
        Ok(rankings) => {
            let limited_rankings = if let Some(limit) = query.limit {
                rankings.into_iter().take(limit as usize).collect()
            } else {
                rankings
            };
            Ok(Json(limited_rankings))
        }
        Err(e) => {
            let period_name = match period {
                RankingPeriod::Daily => "日",
                RankingPeriod::Weekly => "周",
                RankingPeriod::Monthly => "月",
            };
            log::error!("获取{}排名失败: {}", period_name, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
