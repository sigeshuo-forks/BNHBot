use axum::{extract::{Query, State}, response::Json};
use serde::{Deserialize, Serialize};
use crate::services::DatabaseService;

#[derive(Debug, Deserialize)]
pub struct UsernameCheckQuery {
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct UsernameCheckResponse {
    pub available: bool,
    pub message: String,
}

/// 检查用户名是否可用
pub async fn check_username_availability(
    State(database): State<DatabaseService>,
    Query(query): Query<UsernameCheckQuery>,
) -> Json<UsernameCheckResponse> {
    // 验证用户名长度和格式
    let username = query.username.trim();
    
    if username.is_empty() {
        return Json(UsernameCheckResponse {
            available: false,
            message: "用户名不能为空".to_string(),
        });
    }
    
    if username.len() < 2 {
        return Json(UsernameCheckResponse {
            available: false,
            message: "用户名至少需要2个字符".to_string(),
        });
    }
    
    if username.len() > 50 {
        return Json(UsernameCheckResponse {
            available: false,
            message: "用户名不能超过50个字符".to_string(),
        });
    }
    
    // 检查用户名是否已存在
    match database.is_username_exists(username).await {
        Ok(exists) => {
            if exists {
                Json(UsernameCheckResponse {
                    available: false,
                    message: format!("用户名 '{}' 已被使用，请选择其他用户名", username),
                })
            } else {
                Json(UsernameCheckResponse {
                    available: true,
                    message: format!("用户名 '{}' 可以使用", username),
                })
            }
        }
        Err(_) => {
            Json(UsernameCheckResponse {
                available: false,
                message: "检查用户名时发生错误，请稍后重试".to_string(),
            })
        }
    }
}
