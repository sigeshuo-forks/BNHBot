use crate::models::ranking::{RankingEntry, RankingResponse};
use crate::services::RankingService;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RankingQuery {
    pub period: Option<String>,
    pub limit: Option<u32>,
}

/// 获取排名数据
pub async fn get_rankings(
    State((_registration_service, _auth_service, _exchange_service, ranking_service)): State<(
        crate::services::RegistrationService,
        crate::services::AuthService,
        crate::services::ExchangeService,
        RankingService,
    )>,
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

/// 获取特定周期的排名（从固定排名表获取）
pub async fn get_period_rankings(
    State((_registration_service, _auth_service, _exchange_service, ranking_service)): State<(
        crate::services::RegistrationService,
        crate::services::AuthService,
        crate::services::ExchangeService,
        RankingService,
    )>,
    Query(query): Query<RankingQuery>,
) -> Result<Json<Vec<RankingEntry>>, StatusCode> {
    let period_str = query.period.as_deref().unwrap_or("daily");
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    match ranking_service.get_fixed_rankings(period_str, &today).await {
        Ok(rankings) => {
            let limited_rankings = if let Some(limit) = query.limit {
                rankings.into_iter().take(limit as usize).collect()
            } else {
                rankings
            };
            Ok(Json(limited_rankings))
        }
        Err(e) => {
            let period_name = match period_str {
                "daily" => "日",
                "weekly" => "周",
                "monthly" => "月",
                _ => "日",
            };
            log::error!("获取固定{}排名失败: {}", period_name, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
