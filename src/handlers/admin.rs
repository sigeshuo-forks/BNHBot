use crate::models::registration::{Registration, RegistrationStatus, RegistrationStats};
use crate::services::{RegistrationService, AuthService};
use crate::services::auth::{LoginRequest, LoginResponse};
use axum::{
    extract::{Path, State},
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

/// 管理员登录
pub async fn admin_login(
    State((_registration_service, auth_service)): State<(RegistrationService, AuthService)>,
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
    State((registration_service, _auth_service)): State<(RegistrationService, AuthService)>,
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
    State((registration_service, _auth_service)): State<(RegistrationService, AuthService)>,
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
    State((registration_service, _auth_service)): State<(RegistrationService, AuthService)>,
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

    // 执行审核
    match registration_service.review_registration(reg_id, status, request.admin_notes).await {
        Ok(_) => Ok(Json(ReviewResponse {
            success: true,
            message: "审核成功".to_string(),
        })),
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
    State((registration_service, _auth_service)): State<(RegistrationService, AuthService)>,
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
