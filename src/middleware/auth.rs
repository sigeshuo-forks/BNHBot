use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{Json, Response},
};
use serde_json::json;
use crate::services::AuthService;

/// 管理员认证中间件
pub async fn admin_auth_middleware(
    headers: HeaderMap,
    State(auth_service): State<AuthService>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 获取Authorization头部
    let auth_header = headers
        .get("authorization")
        .and_then(|header| header.to_str().ok());

    // 提取token
    let token = match AuthService::extract_token_from_header(auth_header) {
        Some(token) => token,
        None => {
            log::warn!("🚨 未授权访问管理API: 缺少token");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // 验证token
    match auth_service.verify_admin(&token) {
        Ok(true) => {
            // 验证成功，继续处理请求
            Ok(next.run(request).await)
        }
        Ok(false) => {
            log::warn!("🚨 未授权访问管理API: 非管理员用户");
            Err(StatusCode::FORBIDDEN)
        }
        Err(e) => {
            log::warn!("🚨 Token验证失败: {}", e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// API认证错误响应
pub async fn auth_error_handler(
    status: StatusCode,
) -> Json<serde_json::Value> {
    let message = match status {
        StatusCode::UNAUTHORIZED => "未授权访问，请先登录",
        StatusCode::FORBIDDEN => "权限不足，需要管理员权限",
        _ => "认证失败",
    };

    Json(json!({
        "success": false,
        "message": message,
        "code": status.as_u16()
    }))
}
