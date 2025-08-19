use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessImageRequest {
    /// 图片URL（与image_data二选一）
    pub image_url: Option<String>,
    /// Base64编码的图片数据（与image_url二选一）
    pub image_data: Option<String>,
    /// 输出格式，默认为PNG
    pub output_format: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessImageData {
    /// 处理后的图片（Base64编码）
    pub processed_image: String,
    /// 图片格式
    pub format: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonResp<T> {
    pub data: Option<T>,
    pub message: String,
    pub code: i32,
}

impl<T> CommonResp<T> {
    pub fn success(data: T) -> Self {
        Self {
            data: Some(data),
            message: "success".to_string(),
            code: 0,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            data: None,
            message,
            code: -1,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("图片下载失败: {0}")]
    ImageDownload(#[from] reqwest::Error),
    #[error("图片处理失败: {0}")]
    ImageProcessing(#[from] image::ImageError),
    #[error("Base64解码失败: {0}")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("输入错误: {0}")]
    InvalidInput(String),
    #[error("内部服务器错误: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let error_response = CommonResp::<()>::error(self.to_string());

        let status = match self {
            AppError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            AppError::ImageDownload(_) => StatusCode::BAD_REQUEST,
            AppError::Base64Decode(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(error_response)).into_response()
    }
}
