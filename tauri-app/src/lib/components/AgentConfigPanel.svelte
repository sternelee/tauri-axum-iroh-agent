<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { AgentConfig, AgentStatus } from '$lib/agent-types';
  import { createEventDispatcher } from 'svelte';

  export let status: AgentStatus;

  let config: AgentConfig = {
    model: 'claude-3-haiku-20240307',
    preamble: 'You are a helpful assistant.',
    temperature: 0.7,
    max_tokens: 1024,
    enable_tools: true,
  };

  const dispatch = createEventDispatcher();

  async function initializeAgent() {
    dispatch('statusChange', 'initializing');
    try {
      await invoke('initialize_agent', { config });
      dispatch('statusChange', 'idle');
      dispatch('initialized');
    } catch (error) {
      console.error('Failed to initialize agent:', error);
      dispatch('statusChange', 'error');
    }
  }
</script>

<div class="config-panel">
  <h2>Agent Configuration</h2>
  <form on:submit|preventDefault={initializeAgent}>
    <div>
      <label for="model">Model</label>
      <input id="model" type="text" bind:value={config.model} />
    </div>
    <div>
      <label for="preamble">System Prompt</label>
      <textarea id="preamble" bind:value={config.preamble}></textarea>
    </div>
    <div>
      <label for="temperature">Temperature</label>
      <input id="temperature" type="number" step="0.1" bind:value={config.temperature} />
    </div>
    <div>
      <label for="max_tokens">Max Tokens</label>
      <input id="max_tokens" type="number" bind:value={config.max_tokens} />
    </div>
    <div class="checkbox-container">
      <input id="enable_tools" type="checkbox" bind:checked={config.enable_tools} />
      <label for="enable_tools">Enable Tools</label>
    </div>
    <button type="submit" disabled={status === 'initializing' || status === 'idle'}>
      {#if status === 'initializing'}
        Initializing...
      {:else if status === 'idle'}
        Initialized
      {:else}
        Initialize Agent
      {/if}
    </button>
  </form>
</div>

<style>
  .config-panel {
    border: 1px solid #ccc;
    padding: 1rem;
    border-radius: 8px;
    margin-bottom: 1rem;
  }
  div {
    margin-bottom: 0.5rem;
  }
  label {
    display: block;
    margin-bottom: 0.25rem;
  }
  input, textarea {
    width: 100%;
    padding: 0.5rem;
    border-radius: 4px;
    border: 1px solid #ddd;
  }
  .checkbox-container {
    display: flex;
    align-items: center;
  }
  .checkbox-container input {
    width: auto;
    margin-right: 0.5rem;
  }
</style>