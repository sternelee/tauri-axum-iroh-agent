<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import type { AgentMessage, AgentResponse, AgentStatus } from '$lib/agent-types';
  import { onMount, onDestroy } from 'svelte';

  export let status: AgentStatus;
  export let disabled: boolean;

  let messages: AgentMessage[] = [];
  let userInput = '';
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    unlisten = await listen<AgentResponse>('agent://response', (event) => {
      const response = event.payload;
      switch (response.type) {
        case 'MessageChunk':
          // Append chunk to the last assistant message
          const lastMessage = messages[messages.length - 1];
          if (lastMessage && lastMessage.role === 'assistant') {
            lastMessage.content += response.content;
            messages = [...messages]; // Trigger reactivity
          } else {
            messages = [...messages, { role: 'assistant', content: response.content }];
          }
          break;
        case 'ToolCall':
          // Handle tool calls if needed in the future
          console.log('Tool call:', response.tool_call);
          break;
        case 'Error':
          console.error('Agent error:', response.error);
          status = 'error';
          break;
        case 'Done':
          status = 'idle';
          break;
      }
    });
  });

  onDestroy(() => {
    if (unlisten) {
      unlisten();
    }
  });

  async function sendMessage() {
    if (!userInput.trim() || disabled) return;

    const newMessage: AgentMessage = { role: 'user', content: userInput };
    messages = [...messages, newMessage];
    const messageToSend = userInput;
    userInput = '';
    status = 'thinking';

    try {
      await invoke('send_agent_message', { message: messageToSend });
    } catch (error) {
      console.error('Failed to send message:', error);
      status = 'error';
    }
  }
</script>

<div class="chat-container">
  <div class="messages">
    {#each messages as message}
      <div class="message {message.role}">
        <strong>{message.role}:</strong>
        <p>{message.content}</p>
      </div>
    {/each}
  </div>
  <form class="input-form" on:submit|preventDefault={sendMessage}>
    <input
      type="text"
      bind:value={userInput}
      placeholder="Type your message..."
      {disabled}
    />
    <button type="submit" {disabled}
      >{#if status === 'thinking'}Thinking...{:else}Send{/if}</button
    >
  </form>
</div>

<style>
  .chat-container {
    display: flex;
    flex-direction: column;
    height: 500px;
    border: 1px solid #ccc;
    border-radius: 8px;
  }
  .messages {
    flex-grow: 1;
    overflow-y: auto;
    padding: 1rem;
  }
  .message {
    margin-bottom: 1rem;
  }
  .message.user {
    text-align: right;
  }
  .message p {
    display: inline-block;
    padding: 0.5rem 1rem;
    border-radius: 12px;
    background-color: #f1f1f1;
    margin: 0;
  }
  .message.user p {
    background-color: #007bff;
    color: white;
  }
  .input-form {
    display: flex;
    padding: 1rem;
    border-top: 1px solid #ccc;
  }
  input {
    flex-grow: 1;
    padding: 0.5rem;
    border-radius: 4px;
    border: 1px solid #ddd;
  }
  button {
    margin-left: 0.5rem;
  }
</style>