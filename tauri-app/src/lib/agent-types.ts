/**
 * Mirrors the AgentConfig struct in the Rust backend.
 * Used to configure and initialize the agent.
 */
export interface AgentConfig {
  model: string;
  preamble?: string;
  temperature?: number;
  max_tokens?: number;
  enable_tools: boolean;
  history_limit?: number;
}

/**
 * Represents a single message in the conversation history.
 */
export interface AgentMessage {
  role: 'user' | 'assistant' | 'system' | 'tool';
  content: string;
}

/**
 * Represents a tool call requested by the agent.
 */
export interface ToolCall {
  name: string;
  args: Record<string, any>;
}

/**
 * Represents the different types of responses the agent can send.
 * This is a discriminated union, mirroring the Rust enum.
 */
export type AgentResponse =
  | { type: 'MessageChunk'; content: string }
  | { type: 'ToolCall'; tool_call: ToolCall }
  | { type: 'Error'; error: string }
  | { type: 'Done' };

/**
 * Represents the current status of the agent.
 */
export type AgentStatus = 'uninitialized' | 'initializing' | 'idle' | 'thinking' | 'error';