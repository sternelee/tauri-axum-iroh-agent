//! iroh P2P聊天Web API路由

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Json, Sse},
    routing::{get, post},
    Router,
};
use iroh_node::{
    ChatConfig, ChatEvent, CreateRoomRequest, IntegratedClientBuilder, JoinRoomRequest,
    LeaveRoomRequest, MessageType, SendMessageRequest, TransferConfig,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    convert::Infallible,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt as _};
use tracing::{error, info, warn};
use uuid::Uuid;

/// 聊天应用状态
#[derive(Clone)]
pub struct ChatAppState {
    client: Arc<iroh_node::IrohIntegratedClient>,
    chat_sessions: Arc<Mutex<HashMap<String, broadcast::Receiver<ChatEvent>>>>,
}

impl ChatAppState {
    pub async fn new() -> Result<Self, String> {
        let transfer_config = TransferConfig {
            data_root: std::env::temp_dir().join("axum_chat_data"),
            download_dir: Some(std::env::temp_dir().join("axum_chat_downloads")),
            verbose_logging: true,
        };

        let chat_config = ChatConfig {
            user_name: format!("Web用户_{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
            max_message_history: 500,
            enable_file_sharing: true,
        };

        let client = Arc::new(
            IntegratedClientBuilder::new()
                .transfer_config(transfer_config)
                .chat_config(chat_config)
                .enable_chat(true)
                .build()
                .await
                .map_err(|e| format!("创建聊天客户端失败: {}", e))?,
        );

        info!("聊天应用状态初始化成功");

        Ok(Self {
            client,
            chat_sessions: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

/// Web聊天请求类型
#[derive(Deserialize)]
pub struct WebCreateRoomRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct WebJoinRoomRequest {
    pub room_id: String,
    pub user_name: String,
}

#[derive(Deserialize)]
pub struct WebSendMessageRequest {
    pub room_id: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct WebLeaveRoomRequest {
    pub room_id: String,
}

/// Web API响应类型
#[derive(Serialize)]
pub struct WebApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: u64,
}

impl<T> WebApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

/// 创建聊天室
pub async fn create_room(
    State(state): State<ChatAppState>,
    Json(request): Json<WebCreateRoomRequest>,
) -> Result<Json<WebApiResponse<iroh_node::ChatRoom>>, StatusCode> {
    let create_request = CreateRoomRequest {
        name: request.name,
        description: request.description,
    };

    match state.client.create_chat_room(create_request).await {
        Ok(room) => Ok(Json(WebApiResponse::success(room))),
        Err(e) => {
            error!("创建聊天室失败: {}", e);
            Ok(Json(WebApiResponse::error(e.to_string())))
        }
    }
}

/// 加入聊天室
pub async fn join_room(
    State(state): State<ChatAppState>,
    Json(request): Json<WebJoinRoomRequest>,
) -> Result<Json<WebApiResponse<String>>, StatusCode> {
    let join_request = JoinRoomRequest {
        room_id: request.room_id,
        user_name: request.user_name,
    };

    match state.client.join_chat_room(join_request).await {
        Ok(_) => Ok(Json(WebApiResponse::success("成功加入聊天室".to_string()))),
        Err(e) => {
            error!("加入聊天室失败: {}", e);
            Ok(Json(WebApiResponse::error(e.to_string())))
        }
    }
}

/// 发送消息
pub async fn send_message(
    State(state): State<ChatAppState>,
    Json(request): Json<WebSendMessageRequest>,
) -> Result<Json<WebApiResponse<String>>, StatusCode> {
    let send_request = SendMessageRequest {
        room_id: request.room_id,
        content: request.content,
        message_type: MessageType::Text,
    };

    match state.client.send_chat_message(send_request).await {
        Ok(_) => Ok(Json(WebApiResponse::success("消息发送成功".to_string()))),
        Err(e) => {
            error!("发送消息失败: {}", e);
            Ok(Json(WebApiResponse::error(e.to_string())))
        }
    }
}

/// 离开聊天室
pub async fn leave_room(
    State(state): State<ChatAppState>,
    Json(request): Json<WebLeaveRoomRequest>,
) -> Result<Json<WebApiResponse<String>>, StatusCode> {
    let leave_request = LeaveRoomRequest {
        room_id: request.room_id,
    };

    match state.client.leave_chat_room(leave_request).await {
        Ok(_) => Ok(Json(WebApiResponse::success("成功离开聊天室".to_string()))),
        Err(e) => {
            error!("离开聊天室失败: {}", e);
            Ok(Json(WebApiResponse::error(e.to_string())))
        }
    }
}

/// 获取聊天室列表
pub async fn get_rooms(
    State(state): State<ChatAppState>,
) -> Result<Json<WebApiResponse<Vec<iroh_node::ChatRoom>>>, StatusCode> {
    match state.client.get_joined_rooms() {
        Ok(rooms) => Ok(Json(WebApiResponse::success(rooms))),
        Err(e) => {
            error!("获取聊天室列表失败: {}", e);
            Ok(Json(WebApiResponse::error(e.to_string())))
        }
    }
}

/// 获取消息历史
pub async fn get_message_history(
    State(state): State<ChatAppState>,
    Path(room_id): Path<String>,
) -> Result<Json<WebApiResponse<Vec<iroh_node::ChatMessage>>>, StatusCode> {
    match state.client.get_message_history(&room_id) {
        Ok(messages) => Ok(Json(WebApiResponse::success(messages))),
        Err(e) => {
            error!("获取消息历史失败: {}", e);
            Ok(Json(WebApiResponse::error(e.to_string())))
        }
    }
}

/// 创建聊天事件会话
pub async fn create_chat_session(
    State(state): State<ChatAppState>,
) -> Json<WebApiResponse<String>> {
    let session_id = Uuid::new_v4().to_string();
    
    match state.client.subscribe_chat_events() {
        Ok(receiver) => {
            let mut sessions = state.chat_sessions.lock().unwrap();
            sessions.insert(session_id.clone(), receiver);
            Json(WebApiResponse::success(session_id))
        }
        Err(e) => {
            error!("创建聊天会话失败: {}", e);
            Json(WebApiResponse::error(e.to_string()))
        }
    }
}

/// 聊天事件SSE流
pub async fn chat_events_stream(
    State(state): State<ChatAppState>,
    Path(session_id): Path<String>,
) -> Sse<impl tokio_stream::Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    let receiver = {
        let mut sessions = state.chat_sessions.lock().unwrap();
        sessions.remove(&session_id)
    };

    let stream = match receiver {
        Some(rx) => {
            let stream = BroadcastStream::new(rx)
                .map(|result| {
                    match result {
                        Ok(event) => {
                            let json_data = serde_json::to_string(&event).unwrap_or_default();
                            let event_type = match &event {
                                ChatEvent::MessageReceived(_) => "message_received",
                                ChatEvent::UserJoined(_) => "user_joined",
                                ChatEvent::UserLeft { .. } => "user_left",
                                ChatEvent::RoomCreated(_) => "room_created",
                                ChatEvent::RoomUpdated(_) => "room_updated",
                                ChatEvent::ConnectionChanged { .. } => "connection_changed",
                                ChatEvent::Error { .. } => "error",
                            };
                            Ok(axum::response::sse::Event::default()
                                .event(event_type)
                                .data(json_data))
                        }
                        Err(_) => {
                            Ok(axum::response::sse::Event::default()
                                .event("end")
                                .data("stream_ended"))
                        }
                    }
                })
                .take_while(|event| {
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
            warn!("未找到聊天会话ID: {}", session_id);
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

/// 清理聊天会话
pub async fn cleanup_chat_session(
    State(state): State<ChatAppState>,
    Path(session_id): Path<String>,
) -> Json<WebApiResponse<String>> {
    {
        let mut sessions = state.chat_sessions.lock().unwrap();
        sessions.remove(&session_id);
    }
    
    Json(WebApiResponse::success("聊天会话已清理".to_string()))
}

/// 创建聊天路由
pub fn create_chat_routes() -> Router<ChatAppState> {
    Router::new()
        .route("/api/chat/rooms", post(create_room))
        .route("/api/chat/rooms", get(get_rooms))
        .route("/api/chat/rooms/join", post(join_room))
        .route("/api/chat/rooms/leave", post(leave_room))
        .route("/api/chat/messages", post(send_message))
        .route("/api/chat/messages/:room_id", get(get_message_history))
        .route("/api/chat/session", post(create_chat_session))
        .route("/api/chat/events/:session_id", get(chat_events_stream))
        .route("/api/chat/session/:session_id/cleanup", post(cleanup_chat_session))
}