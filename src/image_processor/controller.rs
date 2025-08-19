use axum::{extract::Multipart, Json};
use tracing::info;

use crate::image_processor::{
    model::{AppError, CommonResp, ProcessImageData, ProcessImageRequest},
    service::ImageProcessService,
};

/// 图片处理控制器
pub struct ImageController {
    service: ImageProcessService,
}

impl ImageController {
    pub fn new() -> Self {
        Self {
            service: ImageProcessService::new(),
        }
    }

    /// 处理图像（JSON格式）
    pub async fn process_image(
        Json(request): Json<ProcessImageRequest>,
    ) -> Result<Json<CommonResp<ProcessImageData>>, AppError> {
        info!("收到图像处理请求");

        let controller = Self::new();

        // 获取图像数据
        let image_data = match (&request.image_url, &request.image_data) {
            (Some(url), None) => controller.service.download_image(url).await?,
            (None, Some(data)) => controller.service.decode_base64_image(data)?,
            (Some(_), Some(_)) => {
                return Err(AppError::InvalidInput(
                    "请只提供image_url或image_data中的一个".to_string(),
                ));
            }
            (None, None) => {
                return Err(AppError::InvalidInput(
                    "请提供image_url或image_data".to_string(),
                ));
            }
        };

        // 处理图像
        let processed_image = controller.service.process_image_data(image_data).await?;

        // 确定输出格式
        let output_format = request
            .output_format
            .as_deref()
            .unwrap_or("png")
            .to_lowercase();

        // 转换为Base64
        let base64_image = controller
            .service
            .image_to_base64(processed_image, &output_format)?;

        let response_data = ProcessImageData {
            processed_image: base64_image,
            format: output_format,
        };

        Ok(Json(CommonResp::success(response_data)))
    }

    /// 处理图像（表单格式）
    pub async fn process_image_form(
        mut multipart: Multipart,
    ) -> Result<Json<CommonResp<ProcessImageData>>, AppError> {
        info!("收到表单图像处理请求");

        let controller = Self::new();
        let mut image_data: Option<Vec<u8>> = None;
        let mut output_format = "png".to_string();

        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
        {
            let name = field.name().unwrap_or("");

            match name {
                "image" => {
                    let data = field
                        .bytes()
                        .await
                        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
                    image_data = Some(data.to_vec());
                }
                "format" => {
                    let data = field
                        .bytes()
                        .await
                        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
                    output_format =
                        String::from_utf8(data.to_vec()).unwrap_or_else(|_| "png".to_string());
                }
                _ => {}
            }
        }

        let image_data =
            image_data.ok_or_else(|| AppError::InvalidInput("未找到图片文件".to_string()))?;

        // 处理图像
        let processed_image = controller.service.process_image_data(image_data).await?;

        // 转换为Base64
        let base64_image = controller
            .service
            .image_to_base64(processed_image, &output_format)?;

        let response_data = ProcessImageData {
            processed_image: base64_image,
            format: output_format,
        };

        Ok(Json(CommonResp::success(response_data)))
    }
}

impl Default for ImageController {
    fn default() -> Self {
        Self::new()
    }
}
