use crate::models::registration::{RegistrationRequest, RegistrationResponse};
use crate::services::{RegistrationService, AuthService, ExchangeService};
use axum::{
    extract::State,
    response::Json,
    http::StatusCode,
};


pub async fn handle_registration(
    State((registration_service, _auth_service, _exchange_service)): State<(RegistrationService, AuthService, ExchangeService)>,
    Json(request): Json<RegistrationRequest>,
) -> Result<Json<RegistrationResponse>, StatusCode> {
    // 创建报名
    match registration_service.create_registration(request).await {
        Ok(registration) => Ok(Json(RegistrationResponse {
            success: true,
            message: "报名提交成功！".to_string(),
            registration_id: Some(registration.id),
        })),
        Err(e) => Ok(Json(RegistrationResponse {
            success: false,
            message: format!("报名提交失败: {}", e),
            registration_id: None,
        })),
    }
}
