//! iroh P2P文件传输Web API路由

use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{Json, Sse},
    routing::{get, post},
    Router,
};
use iroh_node::{
    adapters::axum_adapter::{
        AxumAdapter, WebApiResponse, WebDownloadRequest, WebProgressEvent, WebRemoveRequest,
        WebShareResponse, WebUploadRequest,
    },
    ConfigBuilder, ShareResponse,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    convert::Infallible,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt as _};
use tracing::{error, info, warn};
use uuid::Uuid;

/// iroh应用状态
#[derive(Clone)]
pub struct IrohAppState {
    adapter: Arc<AxumAdapter>,
    sessions: Arc<Mutex<HashMap<String, broadcast::Receiver<WebProgressEvent>>>>,
}

impl IrohAppState {
    pub async fn new() -> Result<Self, String> {
        let config = ConfigBuilder::new()
            .data_root(std::env::temp_dir().join("axum_iroh_data"))
            .download_dir(Some(std::env::temp_dir().join("axum_downloads")))
            .verbose_logging(true)
            .build();

        let adapter = Arc::new(
            AxumAdapter::new(config)
                .await
                .map_err(|e| format!("创建iroh适配器失败: {}", e))?,
        );

        info!("iroh适配器初始化成功");

        Ok(Self {
            adapter,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

/// 查询参数
#[derive(Deserialize)]
pub struct ProgressQuery {
    session_id: Option<String>,
}

/// 获取分享代码
pub async fn get_share_code(
    State(state): State<IrohAppState>,
) -> Result<Json<WebApiResponse<WebShareResponse>>, StatusCode> {
    match state.adapter.get_share_code().await {
        Ok(response) => {
            let web_response = WebShareResponse::from(response);
            Ok(Json(WebApiResponse::success(web_response)))
        }
        Err(e) => {
            error!("获取分享代码失败: {}", e);
            Ok(Json(WebApiResponse::error(e.to_string())))
        }
    }
}

/// 下载文件
pub async fn download_files(
    State(state): State<IrohAppState>,
    Query(query): Query<ProgressQuery>,
    Json(request): Json<WebDownloadRequest>,
) -> Result<Json<WebApiResponse<String>>, StatusCode> {
    let download_request = request.into();

    match query.session_id {
        Some(session_id) => {
            // 带进度通知的下载
            match state
                .adapter
                .download_files_with_progress(download_request, session_id.clone())
                .await
            {
                Ok((result, receiver)) => {
                    // 存储接收器供SSE使用
                    {
                        let mut sessions = state.sessions.lock().unwrap();
                        sessions.insert(session_id, receiver);
                    }
                    Ok(Json(WebApiResponse::success(result)))
                }
                Err(e) => {
                    error!("下载文件失败: {}", e);
                    Ok(Json(WebApiResponse::error(e.to_string())))
                }
            }
        }
        None => {
            // 无进度通知的下载
            match state.adapter.download_files(download_request).await {
                Ok(result) => Ok(Json(WebApiResponse::success(result))),
                Err(e) => {
                    error!("下载文件失败: {}", e);
                    Ok(Json(WebApiResponse::error(e.to_string())))
                }
            }
        }
    }
}

/// 上传文件
pub async fn upload_file(
    State(state): State<IrohAppState>,
    Query(query): Query<ProgressQuery>,
    mut multipart: Multipart,
) -> Result<Json<WebApiResponse<String>>, StatusCode> {
    // 处理文件上传
    let mut file_path: Option<PathBuf> = None;

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "file" {
            let file_name = field.file_name().unwrap_or("uploaded_file").to_string();
            let data = field.bytes().await.unwrap();
            
            // 保存到临时目录
            let temp_path = std::env::temp_dir().join(&file_name);
            tokio::fs::write(&temp_path, data).await.unwrap();
            file_path = Some(temp_path);
            break;
        }
    }

    let file_path = match file_path {
        Some(path) => path,
        None => {
            return Ok(Json(WebApiResponse::error("未找到上传文件".to_string())));
        }
    };

    let upload_request = WebUploadRequest {
        file_path: file_path.to_string_lossy().to_string(),
    };
    let upload_request = upload_request.into();

    match query.session_id {
        Some(session_id) => {
            // 带进度通知的上传
            match state
                .adapter
                .upload_file_with_progress(upload_request, session_id.clone())
                .await
            {
                Ok(receiver) => {
                    // 存储接收器供SSE使用
                    {
                        let mut sessions = state.sessions.lock().unwrap();
                        sessions.insert(session_id, receiver);
                    }
                    Ok(Json(WebApiResponse::success("文件上传开始".to_string())))
                }
                Err(e) => {
                    error!("上传文件失败: {}", e);
                    Ok(Json(WebApiResponse::error(e.to_string())))
                }
            }
        }
        None => {
            // 无进度通知的上传
            match state.adapter.upload_file(upload_request).await {
                Ok(_) => Ok(Json(WebApiResponse::success("文件上传成功".to_string()))),
                Err(e) => {
                    error!("上传文件失败: {}", e);
                    Ok(Json(WebApiResponse::error(e.to_string())))
                }
            }
        }
    }
}

/// 删除文件
pub async fn remove_file(
    State(state): State<IrohAppState>,
    Json(request): Json<WebRemoveRequest>,
) -> Result<Json<WebApiResponse<String>>, StatusCode> {
    let remove_request = request.into();

    match state.adapter.remove_file(remove_request).await {
        Ok(_) => Ok(Json(WebApiResponse::success("文件删除成功".to_string()))),
        Err(e) => {
            error!("删除文件失败: {}", e);
            Ok(Json(WebApiResponse::error(e.to_string())))
        }
    }
}

/// 进度事件SSE流
pub async fn progress_stream(
    State(state): State<IrohAppState>,
    Path(session_id): Path<String>,
) -> Sse<impl tokio_stream::Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    let receiver = {
        let mut sessions = state.sessions.lock().unwrap();
        sessions.remove(&session_id)
    };

    let stream = match receiver {
        Some(rx) => {
            let stream = BroadcastStream::new(rx)
                .map(|result| {
                    match result {
                        Ok(event) => {
                            let json_data = serde_json::to_string(&event).unwrap_or_default();
                            Ok(axum::response::sse::Event::default()
                                .event(&event.event_type)
                                .data(json_data))
                        }
                        Err(_) => {
                            // 广播接收器错误，发送结束事件
                            Ok(axum::response::sse::Event::default()
                                .event("end")
                                .data("stream_ended"))
                        }
                    }
                })
                .take_while(|event| {
                    // 当收到结束事件时停止流
                    if let Ok(sse_event) = event {
                        if let Some(event_type) = sse_event.event() {
                            return event_type != "end";
                        }
                    }
                    true
                });
            
            Box::pin(stream) as Box<dyn tokio_stream::Stream<Item = _> + Send>
        }
        None => {
            warn!("未找到会话ID: {}", session_id);
            let error_stream = tokio_stream::once(Ok(
                axum::response::sse::Event::default()
                    .event("error")
                    .data("session_not_found")
            ));
            Box::pin(error_stream) as Box<dyn tokio_stream::Stream<Item = _> + Send>
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive-text"),
    )
}

/// 创建新的进度会话
pub async fn create_session() -> Json<WebApiResponse<String>> {
    let session_id = Uuid::new_v4().to_string();
    Json(WebApiResponse::success(session_id))
}

/// 清理会话
pub async fn cleanup_session(
    State(state): State<IrohAppState>,
    Path(session_id): Path<String>,
) -> Json<WebApiResponse<String>> {
    state.adapter.cleanup_session(&session_id);
    
    {
        let mut sessions = state.sessions.lock().unwrap();
        sessions.remove(&session_id);
    }
    
    Json(WebApiResponse::success("会话已清理".to_string()))
}

/// 创建iroh路由
pub fn create_iroh_routes() -> Router<IrohAppState> {
    Router::new()
        .route("/api/iroh/share", get(get_share_code))
        .route("/api/iroh/download", post(download_files))
        .route("/api/iroh/upload", post(upload_file))
        .route("/api/iroh/remove", post(remove_file))
        .route("/api/iroh/session", post(create_session))
        .route("/api/iroh/session/:session_id/cleanup", post(cleanup_session))
        .route("/api/iroh/progress/:session_id", get(progress_stream))
}