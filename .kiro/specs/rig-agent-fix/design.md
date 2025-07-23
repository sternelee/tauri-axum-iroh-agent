# Design Document

## Overview

This design addresses the incorrect usage of the rig library in the agent implementation by restructuring the code to properly use rig's Agent pattern, correct imports, and proper tool integration. The main changes involve switching from direct CompletionModel usage to proper Agent creation and management.

## Architecture

### Current Issues
- Using `rig::providers::*` imports instead of separate crate imports
- Storing `CompletionModel` instead of `Agent` instances
- Manual message building instead of using rig's conversation patterns
- Direct completion calls instead of agent prompting
- Tool integration not using rig's native tool system

### Proposed Architecture
- Use proper rig provider crates (`rig-openai`, `rig-anthropic`, etc.)
- Store `Agent` instances with proper tool attachments
- Use rig's conversation and prompting patterns
- Integrate tools using rig's tool system
- Proper error handling for rig-specific errors

## Components and Interfaces

### AgentInstance Structure Changes
```rust
struct AgentInstance {
    config: AgentConfig,
    // Change from CompletionModel to Agent
    rig_agent: Box<dyn Agent + Send + Sync>,
    conversation_history: Vec<AgentMessage>,
    created_at: chrono::DateTime<chrono::Utc>,
    last_activity: chrono::DateTime<chrono::Utc>,
}
```

### Provider Creation Updates
- Replace `rig::providers::openai::Client` with `rig_openai::Client`
- Replace `rig::providers::anthropic::Client` with `rig_anthropic::Client`
- Similar changes for cohere and gemini providers
- Use proper model creation patterns from each provider crate

### Agent Creation Pattern
```rust
// Instead of just creating a model, create a full agent
let model = client.model(&config.model);
let mut agent_builder = model.agent();

// Apply configuration
if let Some(system_prompt) = &config.system_prompt {
    agent_builder = agent_builder.preamble(system_prompt);
}

if let Some(temperature) = config.temperature {
    agent_builder = agent_builder.temperature(temperature);
}

// Add tools if enabled
if config.enable_tools {
    let tools = self.tool_manager.get_rig_tools();
    for tool in tools {
        agent_builder = agent_builder.tool(tool);
    }
}

let agent = agent_builder.build();
```

### Chat Method Redesign
- Replace direct completion calls with agent prompting
- Use rig's conversation management
- Handle tool calls through rig's native system
- Proper response extraction

## Data Models

### Tool Integration Changes
The ToolManager needs to provide rig-compatible tools:

```rust
impl ToolManager {
    pub fn get_rig_tools(&self) -> Vec<rig_core::Tool> {
        // Convert internal tool definitions to rig tools
    }
}
```

### Message Handling
- Use rig's message types for conversation building
- Convert between internal AgentMessage and rig messages
- Maintain conversation history in internal format

## Error Handling

### Rig Error Conversion
```rust
impl From<rig_core::Error> for AgentError {
    fn from(err: rig_core::Error) -> Self {
        match err {
            rig_core::Error::ProviderError(_) => AgentError::ModelError(err.to_string()),
            rig_core::Error::ToolError(_) => AgentError::ToolError(err.to_string()),
            _ => AgentError::Other(err.to_string()),
        }
    }
}
```

### Provider-Specific Error Handling
- Handle authentication errors from each provider
- Manage rate limiting and quota errors
- Provide clear error messages for configuration issues

## Testing Strategy

### Unit Tests
- Test agent creation with different providers
- Verify tool integration works correctly
- Test configuration parameter application
- Validate error handling paths

### Integration Tests
- Test actual API calls with mock responses
- Verify tool execution through rig's system
- Test conversation flow with different providers
- Validate error scenarios

### Provider Testing
- Test each supported provider (OpenAI, Anthropic, Cohere, Gemini)
- Verify model parameter application
- Test tool calling with each provider
- Validate response parsing

## Implementation Notes

### Import Changes Required
```rust
// Remove these incorrect imports
use rig::{
    agent::Agent,
    completion::{CompletionModel, Message},
    providers::{openai, anthropic, cohere, gemini},
    tool::Tool,
};

// Add these correct imports
use rig_core::{Agent, Tool};
use rig_openai::Client as OpenAIClient;
use rig_anthropic::Client as AnthropicClient;
use rig_cohere::Client as CohereClient;
use rig_gemini::Client as GeminiClient;
```

### Configuration Mapping
- Map AgentConfig.temperature to rig agent temperature
- Map AgentConfig.max_tokens to rig agent max_tokens
- Map AgentConfig.system_prompt to rig agent preamble
- Handle provider-specific configuration options

### Tool System Integration
- Convert ToolManager tools to rig Tool format
- Handle tool execution through rig's callback system
- Maintain compatibility with existing tool interface
- Support both built-in and custom tools