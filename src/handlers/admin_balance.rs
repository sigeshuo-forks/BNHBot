use axum::{extract::State, response::Json};
use serde::{Serialize, Deserialize};
use crate::services::RankingService;
use log::{info, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectBalanceResponse {
    pub success: bool,
    pub message: String,
    pub collected_count: usize,
    pub failed_count: usize,
    pub errors: Vec<String>,
}

/// 手动收集所有用户的余额数据（管理员功能）
/// 从各个交易所获取最新余额数据并存储到数据库
pub async fn collect_all_balances(
    State(ranking_service): State<RankingService>,
) -> Json<CollectBalanceResponse> {
    info!("管理员手动触发余额收集");
    
    match ranking_service.collect_all_balances().await {
        Ok(result) => {
            info!("余额收集完成 - 成功: {}, 失败: {}", result.collected_count, result.failed_count);
            Json(CollectBalanceResponse {
                success: result.success,
                message: result.message,
                collected_count: result.collected_count as usize,
                failed_count: result.failed_count as usize,
                errors: result.errors,
            })
        }
        Err(e) => {
            error!("余额收集失败: {}", e);
            Json(CollectBalanceResponse {
                success: false,
                message: format!("余额收集失败: {}", e),
                collected_count: 0,
                failed_count: 0,
                errors: vec![e.to_string()],
            })
        }
    }
}
