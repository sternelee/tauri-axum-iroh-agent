//! Axum 适配器实现

use crate::{
    core::{AgentConfig, AgentResponse, ConversationHistory},
    error::{AgentError, AgentResult, ErrorResponse},
    AgentManager,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Json, Response, Sse},
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

/// Axum Agent 适配器
#[derive(Clone)]
pub struct AxumAgentAdapter {
    /// Agent 管理器
    manager: Arc<RwLock<AgentManager>>,
    /// 事件广播器
    event_sender: tokio::sync::broadcast::Sender<ServerSentEvent>,
}

impl AxumAgentAdapter {
    /// 创建新的 Axum 适配器
    pub fn new(default_config: AgentConfig) -> Self {
        let manager = Arc::new(RwLock::new(AgentManager::new(default_config)));
        let (event_sender, _) = tokio::sync::broadcast::channel(1000);
        
        Self {
            manager,
            event_sender,
        }
    }

    /// 获取 Agent 管理器
    pub async fn get_manager(&self) -> tokio::sync::RwLockReadGuard<'_, AgentManager> {
        self.manager.read().await
    }

    /// 获取可变 Agent 管理器
    pub async fn get_manager_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, AgentManager> {
        self.manager.write().await
    }

    /// 发送服务器发送事件
    pub fn send_event(&self, event: ServerSentEvent) {
        let _ = self.event_sender.send(event);
    }

    /// 创建 Axum 路由
    pub fn create_routes(self) -> Router {
        Router::new()
            .route("/agents", post(create_agent_handler))
            .route("/agents", get(list_agents_handler))
            .route("/agents/:agent_id", delete(remove_agent_handler))
            .route("/agents/:agent_id/chat", post(chat_handler))
            .route("/agents/:agent_id/history", get(get_history_handler))
            .route("/agents/:agent_id/history", delete(clear_history_handler))
            .route("/events", get(sse_handler))
            .with_state(self)
    }
}

impl super::AgentAdapter for AxumAgentAdapter {
    async fn chat(&self, agent_id: &str, message: &str) -> AgentResult<AgentResponse> {
        let manager = self.manager.read().await;
        let result = manager.chat(agent_id, message).await;

        // 发送 SSE 事件
        match &result {
            Ok(response) => {
                self.send_event(ServerSentEvent {
                    event: "chat_response".to_string(),
                    data: serde_json::json!({
                        "agent_id": agent_id,
                        "response": response,
                        "timestamp": chrono::Utc::now()
                    }),
                });
            }
            Err(error) => {
                self.send_event(ServerSentEvent {
                    event: "chat_error".to_string(),
                    data: serde_json::json!({
                        "agent_id": agent_id,
                        "error": error.to_string(),
                        "timestamp": chrono::Utc::now()
                    }),
                });
            }
        }

        result
    }

    async fn create_agent(&self, agent_id: String, config: Option<AgentConfig>) -> AgentResult<()> {
        let mut manager = self.manager.write().await;
        let result = manager.create_agent(agent_id.clone(), config.clone()).await;

        if result.is_ok() {
            self.send_event(ServerSentEvent {
                event: "agent_created".to_string(),
                data: serde_json::json!({
                    "agent_id": agent_id,
                    "config": config,
                    "timestamp": chrono::Utc::now()
                }),
            });
        }

        result
    }

    async fn remove_agent(&self, agent_id: &str) -> AgentResult<bool> {
        let mut manager = self.manager.write().await;
        let result = manager.remove_agent(agent_id).await;

        if result {
            self.send_event(ServerSentEvent {
                event: "agent_removed".to_string(),
                data: serde_json::json!({
                    "agent_id": agent_id,
                    "timestamp": chrono::Utc::now()
                }),
            });
        }

        Ok(result)
    }

    async fn list_agents(&self) -> AgentResult<Vec<String>> {
        let manager = self.manager.read().await;
        Ok(manager.list_agents().await)
    }
}

/// 服务器发送事件
#[derive(Debug, Clone, Serialize)]
pub struct ServerSentEvent {
    pub event: String,
    pub data: serde_json::Value,
}

/// API 请求类型
#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub agent_id: String,
    pub config: Option<AgentConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// API 响应类型
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: chrono::Utc::now(),
        }
    }
}

impl<T> From<AgentResult<T>> for ApiResponse<T> {
    fn from(result: AgentResult<T>) -> Self {
        match result {
            Ok(data) => Self::success(data),
            Err(error) => Self::error(error.to_string()),
        }
    }
}

/// Axum 路由处理器
async fn create_agent_handler(
    State(adapter): State<AxumAgentAdapter>,
    Json(request): Json<CreateAgentRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ErrorResponse>)> {
    let result = adapter.create_agent(request.agent_id, request.config).await;
    
    match result {
        Ok(data) => Ok(Json(ApiResponse::success(data))),
        Err(error) => {
            let status = match error {
                AgentError::Configuration(_) => StatusCode::BAD_REQUEST,
                AgentError::Permission(_) => StatusCode::FORBIDDEN,
                AgentError::RateLimit => StatusCode::TOO_MANY_REQUESTS,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Err((status, Json(ErrorResponse::from_error(&error))))
        }
    }
}

async fn list_agents_handler(
    State(adapter): State<AxumAgentAdapter>,
) -> Result<Json<ApiResponse<Vec<String>>>, (StatusCode, Json<ErrorResponse>)> {
    let result = adapter.list_agents().await;
    
    match result {
        Ok(data) => Ok(Json(ApiResponse::success(data))),
        Err(error) => {
            let status = StatusCode::INTERNAL_SERVER_ERROR;
            Err((status, Json(ErrorResponse::from_error(&error))))
        }
    }
}

async fn remove_agent_handler(
    State(adapter): State<AxumAgentAdapter>,
    Path(agent_id): Path<String>,
) -> Result<Json<ApiResponse<bool>>, (StatusCode, Json<ErrorResponse>)> {
    let result = adapter.remove_agent(&agent_id).await;
    
    match result {
        Ok(data) => Ok(Json(ApiResponse::success(data))),
        Err(error) => {
            let status = match error {
                AgentError::AgentNotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Err((status, Json(ErrorResponse::from_error(&error))))
        }
    }
}

async fn chat_handler(
    State(adapter): State<AxumAgentAdapter>,
    Path(agent_id): Path<String>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<ApiResponse<AgentResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let result = adapter.chat(&agent_id, &request.message).await;
    
    match result {
        Ok(data) => Ok(Json(ApiResponse::success(data))),
        Err(error) => {
            let status = match error {
                AgentError::AgentNotFound(_) => StatusCode::NOT_FOUND,
                AgentError::RateLimit => StatusCode::TOO_MANY_REQUESTS,
                AgentError::InsufficientTokens => StatusCode::PAYMENT_REQUIRED,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Err((status, Json(ErrorResponse::from_error(&error))))
        }
    }
}

async fn get_history_handler(
    State(adapter): State<AxumAgentAdapter>,
    Path(agent_id): Path<String>,
    Query(params): Query<QueryParams>,
) -> Result<Json<ApiResponse<ConversationHistory>>, (StatusCode, Json<ErrorResponse>)> {
    let manager = adapter.get_manager().await;
    let result = manager.get_conversation_history(&agent_id).await;
    
    match result {
        Ok(mut history) => {
            // 应用分页参数
            if let Some(offset) = params.offset {
                if offset < history.messages.len() {
                    history.messages = history.messages.into_iter().skip(offset).collect();
                } else {
                    history.messages.clear();
                }
            }
            
            if let Some(limit) = params.limit {
                history.messages.truncate(limit);
            }
            
            Ok(Json(ApiResponse::success(history)))
        }
        Err(error) => {
            let status = match error {
                AgentError::AgentNotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Err((status, Json(ErrorResponse::from_error(&error))))
        }
    }
}

async fn clear_history_handler(
    State(adapter): State<AxumAgentAdapter>,
    Path(agent_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ErrorResponse>)> {
    let manager = adapter.get_manager().await;
    let result = manager.clear_conversation_history(&agent_id).await;
    
    match result {
        Ok(data) => {
            adapter.send_event(ServerSentEvent {
                event: "history_cleared".to_string(),
                data: serde_json::json!({
                    "agent_id": agent_id,
                    "timestamp": chrono::Utc::now()
                }),
            });
            Ok(Json(ApiResponse::success(data)))
        }
        Err(error) => {
            let status = match error {
                AgentError::AgentNotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Err((status, Json(ErrorResponse::from_error(&error))))
        }
    }
}

async fn sse_handler(
    State(adapter): State<AxumAgentAdapter>,
) -> Sse<impl tokio_stream::Stream<Item = Result<axum::response::sse::Event, axum::response::sse::Event>>> {
    let receiver = adapter.event_sender.subscribe();
    let stream = BroadcastStream::new(receiver)
        .map(|result| {
            match result {
                Ok(event) => {
                    let data = serde_json::to_string(&event.data).unwrap_or_default();
                    Ok(axum::response::sse::Event::default()
                        .event(event.event)
                        .data(data))
                }
                Err(_) => {
                    // 处理广播接收错误
                    Ok(axum::response::sse::Event::default()
                        .event("error")
                        .data("广播接收错误"))
                }
            }
        });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive"),
    )
}

/// 健康检查处理器
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// 创建完整的 API 路由
pub fn create_api_routes(adapter: AxumAgentAdapter) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1", adapter.create_routes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        let json_value = response.0;
        assert_eq!(json_value["status"], "healthy");
    }

    #[tokio::test]
    async fn test_axum_adapter_creation() {
        let config = AgentConfig::default();
        let adapter = AxumAgentAdapter::new(config);
        
        let agents = adapter.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }
}