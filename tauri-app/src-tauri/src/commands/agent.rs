use rig_agent::{AgentConfig, AgentError, AgentManager, AgentResult, TauriAgentAdapter};
use std::sync::Arc;
use tauri::{async_runtime::Mutex, AppHandle, State};
use tracing::info;

/// Tauri-managed state for the rig-agent.
/// We use a Mutex-guarded Option because the AgentManager is initialized asynchronously
/// based on frontend configuration.
pub struct AgentState(pub Arc<Mutex<Option<AgentManager<TauriAgentAdapter>>>>);

impl AgentState {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }
}

/// Initializes the AgentManager with a given configuration.
/// This command should be called from the frontend before any other agent operations.
#[tauri::command]
pub async fn initialize_agent(
    config: AgentConfig,
    state: State<'_, AgentState>,
    app_handle: AppHandle,
) -> AgentResult<()> {
    info!("Initializing agent with config: {:?}", config);
    let adapter = TauriAgentAdapter::new(app_handle);
    let agent_manager = AgentManager::new_with_adapter(config, adapter).await?;
    let mut guard = state.0.lock().await;
    *guard = Some(agent_manager);
    info!("Agent initialized successfully");
    Ok(())
}

/// Sends a message to the agent and starts the conversation stream.
/// The agent will send back responses (AgentResponse) via Tauri events.
#[tauri::command]
pub async fn send_agent_message(message: String, state: State<'_, AgentState>) -> AgentResult<()> {
    let mut guard = state.0.lock().await;
    if let Some(manager) = guard.as_mut() {
        // The `send_message` method will internally use the TauriAgentAdapter
        // to stream responses back to the frontend.
        manager.send_message(message.into()).await?;
        Ok(())
    } else {
        Err(AgentError::NotInitialized)
    }
}