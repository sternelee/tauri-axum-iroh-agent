# Implementation Plan

- [ ] 1. Update imports and dependencies
  - Replace incorrect rig imports with proper provider crate imports
  - Add missing rig-core imports for Agent trait
  - Update Cargo.toml if needed for correct rig crate versions
  - _Requirements: 1.1, 1.2_

- [ ] 2. Fix AgentInstance structure and provider creation
  - [ ] 2.1 Update AgentInstance to store Agent instead of CompletionModel
    - Change rig_agent field type from CompletionModel to Agent trait object
    - Update all references to use Agent methods instead of CompletionModel
    - _Requirements: 1.3_
  
  - [ ] 2.2 Rewrite create_provider method to use correct rig patterns
    - Replace rig::providers imports with individual provider crate usage
    - Use proper client creation patterns for each provider
    - Create Agent instances instead of just models
    - Apply configuration parameters (temperature, max_tokens, system_prompt) to agents
    - _Requirements: 1.1, 1.2, 5.1, 5.2, 5.3_

- [ ] 3. Implement proper tool integration with rig
  - [ ] 3.1 Update ToolManager to provide rig-compatible tools
    - Add method to convert internal tools to rig Tool format
    - Implement tool callback system for rig integration
    - _Requirements: 2.1_
  
  - [ ] 3.2 Integrate tools into agent creation process
    - Conditionally attach tools to agents based on enable_tools config
    - Handle tool execution through rig's native system
    - _Requirements: 2.1, 2.3_

- [ ] 4. Rewrite chat method to use rig Agent patterns
  - [ ] 4.1 Replace direct completion calls with agent prompting
    - Use Agent.prompt() or similar methods instead of completion()
    - Remove manual message building in favor of rig's conversation patterns
    - _Requirements: 3.1, 3.2_
  
  - [ ] 4.2 Update response handling and tool call processing
    - Extract content and tool calls from rig agent responses
    - Handle tool execution through rig's callback system instead of manual execution
    - Update conversation history management to work with new patterns
    - _Requirements: 2.2, 3.3_

- [ ] 5. Add proper error handling for rig-specific errors
  - Implement From trait to convert rig errors to AgentError
  - Add specific error handling for provider authentication failures
  - Handle rate limiting and API quota errors appropriately
  - _Requirements: 4.1, 4.2, 4.3_

- [ ] 6. Update tests to work with new rig integration
  - [ ] 6.1 Fix unit tests for agent creation and management
    - Update test expectations for Agent instead of CompletionModel
    - Test configuration parameter application
    - _Requirements: 1.1, 1.2, 1.3_
  
  - [ ] 6.2 Update integration tests for tool functionality
    - Test tool integration through rig's system
    - Verify tool execution works with different providers
    - _Requirements: 2.1, 2.2_
  
  - [ ] 6.3 Add tests for error handling scenarios
    - Test rig error conversion to AgentError
    - Verify proper handling of provider-specific errors
    - _Requirements: 4.1, 4.2, 4.3_

- [ ] 7. Verify and test with all supported providers
  - Test agent creation and chat functionality with OpenAI
  - Test agent creation and chat functionality with Anthropic
  - Test agent creation and chat functionality with Cohere
  - Test agent creation and chat functionality with Gemini
  - Ensure tool calling works consistently across providers
  - _Requirements: 1.1, 1.2, 2.1, 3.1_