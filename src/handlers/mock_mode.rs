use axum::{extract::State, http::StatusCode, response::Json};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockModeRequest {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockModeResponse {
    pub enabled: bool,
    pub message: String,
}

// 全局Mock模式状态
lazy_static! {
    static ref MOCK_MODE_ENABLED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

/// 获取Mock模式状态（管理员接口）
pub async fn get_mock_mode(
    State(_): State<(
        crate::services::RegistrationService,
        crate::services::AuthService,
        crate::services::ExchangeService,
    )>,
) -> Result<Json<MockModeResponse>, StatusCode> {
    let enabled = *MOCK_MODE_ENABLED.lock().unwrap();

    Ok(Json(MockModeResponse {
        enabled,
        message: if enabled {
            "演示模式已开启".to_string()
        } else {
            "演示模式已关闭".to_string()
        },
    }))
}

/// 获取Mock模式状态（公开接口，供排名页面使用）
pub async fn get_mock_mode_public() -> Result<Json<MockModeResponse>, StatusCode> {
    let enabled = *MOCK_MODE_ENABLED.lock().unwrap();

    Ok(Json(MockModeResponse {
        enabled,
        message: if enabled {
            "演示模式已开启".to_string()
        } else {
            "演示模式已关闭".to_string()
        },
    }))
}

/// 设置Mock模式状态
pub async fn set_mock_mode(
    State(_): State<(
        crate::services::RegistrationService,
        crate::services::AuthService,
        crate::services::ExchangeService,
    )>,
    Json(request): Json<MockModeRequest>,
) -> Result<Json<MockModeResponse>, StatusCode> {
    {
        let mut enabled = MOCK_MODE_ENABLED.lock().unwrap();
        *enabled = request.enabled;
    }

    Ok(Json(MockModeResponse {
        enabled: request.enabled,
        message: if request.enabled {
            "演示模式已开启".to_string()
        } else {
            "演示模式已关闭".to_string()
        },
    }))
}

/// 检查是否启用Mock模式
pub fn is_mock_mode_enabled() -> bool {
    *MOCK_MODE_ENABLED.lock().unwrap()
}
