use axum::{
    routing::{get, post},
    Json, Router,
};
use tower_http::cors::CorsLayer;
use tracing::info;

mod image_processor;
use image_processor::{controller::ImageController, model::CommonResp};

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 创建路由
    let app = Router::new()
        .route("/", get(health_check))
        .route("/health", get(health_check))
        .route("/process", post(ImageController::process_image))
        .route("/process-form", post(ImageController::process_image_form))
        .layer(CorsLayer::permissive());

    info!("启动图像处理服务在端口 3000");

    // 启动服务器
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// 健康检查端点
async fn health_check() -> Json<CommonResp<serde_json::Value>> {
    let health_data = serde_json::json!({
        "status": "ok",
        "service": "图像处理服务",
        "version": "0.1.0"
    });
    Json(CommonResp::success(health_data))
}
