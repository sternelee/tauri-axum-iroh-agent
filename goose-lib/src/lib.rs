use futures::StreamExt;
use goose::agents::{Agent, AgentEvent};
use goose::message::Message;
use goose::model::ModelConfig;
use goose::providers::{
    anthropic::AnthropicProvider, databricks::DatabricksProvider, google::GoogleProvider,
    openai::OpenAIProvider,
};
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// Re-exports for convenience
pub use goose::model::ModelConfig;
pub use goose::providers::{
    anthropic::AnthropicProvider, databricks::DatabricksProvider, google::GoogleProvider,
    openai::OpenAIProvider,
};

// Core error types
#[derive(Debug, Error)]
pub enum GooseError {
    #[error("Agent initialization failed: {0}")]
    InitializationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Message processing error: {0}")]
    MessageError(String),

    #[error("Streaming error: {0}")]
    StreamError(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Goose framework error: {0}")]
    GooseFrameworkError(String),
}

/// Configuration for the goose agent
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Model-specific configuration
    pub model_config: ModelConfig,
    /// Provider-specific configuration (OpenAI, Anthropic, Google, Databricks, etc.)
    pub provider_config: ProviderConfig,
    /// Chat behavior configuration
    pub chat_config: ChatConfig,
}

/// Configuration for chat behavior
#[derive(Debug, Clone)]
pub struct ChatConfig {
    /// Maximum number of messages to keep in conversation history
    pub max_history_length: usize,
    /// Whether to enable streaming responses
    pub enable_streaming: bool,
    /// Timeout for requests in seconds
    pub timeout_seconds: u64,
    /// Optional system prompt to set context for the conversation
    pub system_prompt: Option<String>,
}

/// Configuration for different AI providers
#[derive(Debug, Clone)]
pub enum ProviderConfig {
    /// Databricks provider configuration
    Databricks {
        /// Databricks endpoint URL
        endpoint: String,
        /// Authentication token
        token: String,
        /// Model name to use
        model: String,
    },
    /// OpenAI provider configuration
    OpenAI {
        /// OpenAI API key
        api_key: String,
        /// Model name (e.g., "gpt-4", "gpt-3.5-turbo")
        model: String,
        /// Optional custom base URL (for OpenAI-compatible APIs)
        base_url: Option<String>,
        /// Optional organization ID
        organization: Option<String>,
    },
    /// Anthropic (Claude) provider configuration
    Anthropic {
        /// Anthropic API key
        api_key: String,
        /// Model name (e.g., "claude-3-sonnet-20240229", "claude-3-haiku-20240307")
        model: String,
        /// Optional custom base URL
        base_url: Option<String>,
    },
    /// Google (Gemini) provider configuration
    Google {
        /// Google API key
        api_key: String,
        /// Model name (e.g., "gemini-pro", "gemini-pro-vision")
        model: String,
        /// Optional custom base URL
        base_url: Option<String>,
    },
    // Add more providers as needed
}

impl ProviderConfig {
    /// Get the name of the provider for logging/debugging purposes
    pub fn provider_name(&self) -> &'static str {
        match self {
            ProviderConfig::Databricks { .. } => "Databricks",
            ProviderConfig::OpenAI { .. } => "OpenAI",
            ProviderConfig::Anthropic { .. } => "Anthropic",
            ProviderConfig::Google { .. } => "Google",
        }
    }

    /// Get the model name being used
    pub fn model_name(&self) -> &str {
        match self {
            ProviderConfig::Databricks { model, .. } => model,
            ProviderConfig::OpenAI { model, .. } => model,
            ProviderConfig::Anthropic { model, .. } => model,
            ProviderConfig::Google { model, .. } => model,
        }
    }
}

// Message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub content: String,
    pub role: MessageRole,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

// Streaming response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResponse {
    pub chunk: String,
    pub is_complete: bool,
    pub error: Option<String>,
}

// Conversation structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: u64,
    pub updated_at: u64,
}

// Default implementations
impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            max_history_length: 100,
            enable_streaming: true,
            timeout_seconds: 30,
            system_prompt: None,
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model_config: ModelConfig::default(),
            provider_config: ProviderConfig::OpenAI {
                api_key: "".to_string(),
                model: "gpt-4".to_string(),
                base_url: None,
                organization: None,
            },
            chat_config: ChatConfig::default(),
        }
    }
}

impl AgentConfig {
    /// Create a new AgentConfig with OpenAI provider
    pub fn openai(api_key: String, model: String) -> Self {
        Self {
            model_config: ModelConfig::default(),
            provider_config: ProviderConfig::OpenAI {
                api_key,
                model,
                base_url: None,
                organization: None,
            },
            chat_config: ChatConfig::default(),
        }
    }

    /// Create a new AgentConfig with OpenAI provider and custom base URL
    pub fn openai_with_base_url(api_key: String, model: String, base_url: String) -> Self {
        Self {
            model_config: ModelConfig::default(),
            provider_config: ProviderConfig::OpenAI {
                api_key,
                model,
                base_url: Some(base_url),
                organization: None,
            },
            chat_config: ChatConfig::default(),
        }
    }

    /// Create a new AgentConfig with Anthropic provider
    pub fn anthropic(api_key: String, model: String) -> Self {
        Self {
            model_config: ModelConfig::default(),
            provider_config: ProviderConfig::Anthropic {
                api_key,
                model,
                base_url: None,
            },
            chat_config: ChatConfig::default(),
        }
    }

    /// Create a new AgentConfig with Google provider
    pub fn google(api_key: String, model: String) -> Self {
        Self {
            model_config: ModelConfig::default(),
            provider_config: ProviderConfig::Google {
                api_key,
                model,
                base_url: None,
            },
            chat_config: ChatConfig::default(),
        }
    }

    /// Create a new AgentConfig with Databricks provider
    pub fn databricks(endpoint: String, token: String, model: String) -> Self {
        Self {
            model_config: ModelConfig::default(),
            provider_config: ProviderConfig::Databricks {
                endpoint,
                token,
                model,
            },
            chat_config: ChatConfig::default(),
        }
    }

    /// Set custom chat configuration
    pub fn with_chat_config(mut self, chat_config: ChatConfig) -> Self {
        self.chat_config = chat_config;
        self
    }

    /// Set custom model configuration
    pub fn with_model_config(mut self, model_config: ModelConfig) -> Self {
        self.model_config = model_config;
        self
    }
}

impl ChatMessage {
    pub fn new(content: String, role: MessageRole) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            role,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    pub fn user_message(content: String) -> Self {
        Self::new(content, MessageRole::User)
    }

    pub fn assistant_message(content: String) -> Self {
        Self::new(content, MessageRole::Assistant)
    }

    pub fn system_message(content: String) -> Self {
        Self::new(content, MessageRole::System)
    }
}

impl StreamResponse {
    pub fn chunk(chunk: String) -> Self {
        Self {
            chunk,
            is_complete: false,
            error: None,
        }
    }

    pub fn complete() -> Self {
        Self {
            chunk: String::new(),
            is_complete: true,
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            chunk: String::new(),
            is_complete: true,
            error: Some(error),
        }
    }
}
// Core Agent Manager
pub struct GooseAgentManager {
    agent: OnceCell<Agent>,
    runtime: Arc<Runtime>,
    config: AgentConfig,
}

impl GooseAgentManager {
    pub fn new(config: AgentConfig) -> Result<Self, GooseError> {
        let runtime = Runtime::new().map_err(|e| {
            GooseError::RuntimeError(format!("Failed to create tokio runtime: {}", e))
        })?;

        Ok(Self {
            agent: OnceCell::new(),
            runtime: Arc::new(runtime),
            config,
        })
    }

    pub fn get_agent(&self) -> Result<&Agent, GooseError> {
        self.agent.get_or_try_init(|| {
            info!(
                "Initializing goose agent with {} provider using model: {}",
                self.config.provider_config.provider_name(),
                self.config.provider_config.model_name()
            );

            // Create the agent based on provider configuration
            let agent = match &self.config.provider_config {
                ProviderConfig::Databricks {
                    endpoint,
                    token,
                    model: _,
                } => {
                    let provider = DatabricksProvider::new(endpoint.clone(), token.clone());

                    Agent::builder()
                        .with_provider(Box::new(provider))
                        .build()
                        .map_err(|e| {
                            GooseError::InitializationError(format!(
                                "Failed to build Databricks agent: {}",
                                e
                            ))
                        })?
                }
                ProviderConfig::OpenAI {
                    api_key,
                    model,
                    base_url,
                    organization,
                } => {
                    let mut provider = OpenAIProvider::new(api_key.clone(), model.clone());

                    if let Some(base_url) = base_url {
                        provider = provider.with_base_url(base_url.clone());
                    }

                    if let Some(org) = organization {
                        provider = provider.with_organization(org.clone());
                    }

                    Agent::builder()
                        .with_provider(Box::new(provider))
                        .build()
                        .map_err(|e| {
                            GooseError::InitializationError(format!(
                                "Failed to build OpenAI agent: {}",
                                e
                            ))
                        })?
                }
                ProviderConfig::Anthropic {
                    api_key,
                    model,
                    base_url,
                } => {
                    let mut provider = AnthropicProvider::new(api_key.clone(), model.clone());

                    if let Some(base_url) = base_url {
                        provider = provider.with_base_url(base_url.clone());
                    }

                    Agent::builder()
                        .with_provider(Box::new(provider))
                        .build()
                        .map_err(|e| {
                            GooseError::InitializationError(format!(
                                "Failed to build Anthropic agent: {}",
                                e
                            ))
                        })?
                }
                ProviderConfig::Google {
                    api_key,
                    model,
                    base_url,
                } => {
                    let mut provider = GoogleProvider::new(api_key.clone(), model.clone());

                    if let Some(base_url) = base_url {
                        provider = provider.with_base_url(base_url.clone());
                    }

                    Agent::builder()
                        .with_provider(Box::new(provider))
                        .build()
                        .map_err(|e| {
                            GooseError::InitializationError(format!(
                                "Failed to build Google agent: {}",
                                e
                            ))
                        })?
                }
            };

            info!("Goose agent initialized successfully");
            Ok(agent)
        })
    }

    pub async fn send_message(&self, message: &str) -> Result<String, GooseError> {
        let agent = self.get_agent()?;

        debug!("Sending message to agent: {}", message);

        // Create a user message
        let user_message = Message::user().with_text(message);

        // Send message and get response
        let mut response_stream = agent
            .run(&user_message)
            .await
            .map_err(|e| GooseError::MessageError(format!("Failed to send message: {}", e)))?;

        let mut full_response = String::new();
        while let Some(event) = response_stream.next().await {
            match event {
                AgentEvent::MessageChunk(chunk) => {
                    full_response.push_str(&chunk);
                }
                AgentEvent::Error(err) => {
                    return Err(GooseError::MessageError(format!("Agent error: {}", err)));
                }
                _ => {} // Handle other events as needed
            }
        }

        debug!("Received response from agent");
        Ok(full_response)
    }

    pub async fn send_message_stream(
        &self,
        message: &str,
    ) -> Result<impl futures::Stream<Item = Result<String, GooseError>>, GooseError> {
        let agent = self.get_agent()?;

        debug!("Sending streaming message to agent: {}", message);

        // Create a user message
        let user_message = Message::user().with_text(message);

        // Send message and get streaming response
        let stream = agent.run(&user_message).await.map_err(|e| {
            GooseError::StreamError(format!("Failed to start message stream: {}", e))
        })?;

        // Transform the stream to handle errors properly
        let error_handled_stream = stream.map(|event| {
            match event {
                AgentEvent::MessageChunk(chunk) => Ok(chunk),
                AgentEvent::Error(err) => {
                    Err(GooseError::StreamError(format!("Stream error: {}", err)))
                }
                _ => Ok(String::new()), // Handle other event types as empty strings
            }
        });

        debug!("Started streaming response from agent");
        Ok(error_handled_stream)
    }

    pub async fn send_message_with_retry(
        &self,
        message: &str,
        max_retries: u32,
    ) -> Result<String, GooseError> {
        let mut attempts = 0;
        let mut delay = Duration::from_millis(100);

        loop {
            match self.send_message(message).await {
                Ok(response) => {
                    if attempts > 0 {
                        info!("Message succeeded after {} retries", attempts);
                    }
                    return Ok(response);
                }
                Err(e) if attempts < max_retries => {
                    attempts += 1;
                    warn!(
                        "Message attempt {} failed, retrying in {:?}: {}",
                        attempts, delay, e
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                }
                Err(e) => {
                    error!("Message failed after {} attempts: {}", attempts + 1, e);
                    return Err(e);
                }
            }
        }
    }
}

// Thread-safe singleton instance
static GLOBAL_AGENT_MANAGER: OnceCell<Arc<GooseAgentManager>> = OnceCell::new();

pub fn get_global_agent_manager() -> Result<Arc<GooseAgentManager>, GooseError> {
    GLOBAL_AGENT_MANAGER.get().cloned().ok_or_else(|| {
        GooseError::InitializationError(
            "Global agent manager not initialized. Call initialize_global_agent_manager first."
                .to_string(),
        )
    })
}

pub fn initialize_global_agent_manager(
    config: AgentConfig,
) -> Result<Arc<GooseAgentManager>, GooseError> {
    let manager = Arc::new(GooseAgentManager::new(config)?);
    GLOBAL_AGENT_MANAGER.set(manager.clone()).map_err(|_| {
        GooseError::InitializationError("Global agent manager already initialized".to_string())
    })?;

    info!("Global agent manager initialized");
    Ok(manager)
}

pub fn reset_global_agent_manager() {
    // This is mainly for testing purposes
    // Note: OnceCell doesn't have a reset method, so this is a no-op in production
    // In tests, we would need to use a different approach or accept that the global state persists
    warn!("Global agent manager reset requested - this is only supported in test environments");
}
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> AgentConfig {
        AgentConfig::openai("test_api_key".to_string(), "gpt-4".to_string())
    }

    fn create_test_databricks_config() -> AgentConfig {
        AgentConfig::databricks(
            "https://test.databricks.com".to_string(),
            "test_token".to_string(),
            "test_model".to_string(),
        )
    }

    fn create_test_anthropic_config() -> AgentConfig {
        AgentConfig::anthropic(
            "test_api_key".to_string(),
            "claude-3-sonnet-20240229".to_string(),
        )
    }

    fn create_test_google_config() -> AgentConfig {
        AgentConfig::google("test_api_key".to_string(), "gemini-pro".to_string())
    }

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.chat_config.max_history_length, 100);
        assert!(config.chat_config.enable_streaming);
        assert_eq!(config.chat_config.timeout_seconds, 30);
        assert!(config.chat_config.system_prompt.is_none());
    }

    #[test]
    fn test_chat_message_creation() {
        let message = ChatMessage::user_message("Hello, world!".to_string());
        assert_eq!(message.content, "Hello, world!");
        assert!(matches!(message.role, MessageRole::User));
        assert!(!message.id.is_empty());
        assert!(message.timestamp > 0);
    }

    #[test]
    fn test_stream_response_creation() {
        let chunk_response = StreamResponse::chunk("Hello".to_string());
        assert_eq!(chunk_response.chunk, "Hello");
        assert!(!chunk_response.is_complete);
        assert!(chunk_response.error.is_none());

        let complete_response = StreamResponse::complete();
        assert!(complete_response.is_complete);
        assert!(complete_response.chunk.is_empty());

        let error_response = StreamResponse::error("Test error".to_string());
        assert!(error_response.is_complete);
        assert_eq!(error_response.error.as_ref().unwrap(), "Test error");
    }

    #[test]
    fn test_agent_manager_creation() {
        let config = create_test_config();
        let manager = GooseAgentManager::new(config);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_openai_config_creation() {
        let config = AgentConfig::openai("test_key".to_string(), "gpt-4".to_string());
        match config.provider_config {
            ProviderConfig::OpenAI { api_key, model, .. } => {
                assert_eq!(api_key, "test_key");
                assert_eq!(model, "gpt-4");
            }
            _ => panic!("Expected OpenAI provider config"),
        }
    }

    #[test]
    fn test_anthropic_config_creation() {
        let config = AgentConfig::anthropic(
            "test_key".to_string(),
            "claude-3-sonnet-20240229".to_string(),
        );
        match config.provider_config {
            ProviderConfig::Anthropic { api_key, model, .. } => {
                assert_eq!(api_key, "test_key");
                assert_eq!(model, "claude-3-sonnet-20240229");
            }
            _ => panic!("Expected Anthropic provider config"),
        }
    }

    #[test]
    fn test_google_config_creation() {
        let config = AgentConfig::google("test_key".to_string(), "gemini-pro".to_string());
        match config.provider_config {
            ProviderConfig::Google { api_key, model, .. } => {
                assert_eq!(api_key, "test_key");
                assert_eq!(model, "gemini-pro");
            }
            _ => panic!("Expected Google provider config"),
        }
    }

    #[test]
    fn test_databricks_config_creation() {
        let config = AgentConfig::databricks(
            "https://test.databricks.com".to_string(),
            "test_token".to_string(),
            "test_model".to_string(),
        );
        match config.provider_config {
            ProviderConfig::Databricks {
                endpoint,
                token,
                model,
            } => {
                assert_eq!(endpoint, "https://test.databricks.com");
                assert_eq!(token, "test_token");
                assert_eq!(model, "test_model");
            }
            _ => panic!("Expected Databricks provider config"),
        }
    }

    #[test]
    fn test_config_with_custom_chat_config() {
        let custom_chat_config = ChatConfig {
            max_history_length: 50,
            enable_streaming: false,
            timeout_seconds: 60,
            system_prompt: Some("You are a helpful assistant.".to_string()),
        };

        let config = AgentConfig::openai("test_key".to_string(), "gpt-4".to_string())
            .with_chat_config(custom_chat_config.clone());

        assert_eq!(config.chat_config.max_history_length, 50);
        assert!(!config.chat_config.enable_streaming);
        assert_eq!(config.chat_config.timeout_seconds, 60);
        assert_eq!(
            config.chat_config.system_prompt.as_ref().unwrap(),
            "You are a helpful assistant."
        );
    }

    // Note: Integration tests with actual agent initialization would require
    // valid API credentials and would be better placed in integration tests
}
