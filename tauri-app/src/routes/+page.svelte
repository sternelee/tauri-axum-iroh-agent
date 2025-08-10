<script lang="ts">
  import AgentConfigPanel from '$lib/components/AgentConfigPanel.svelte';
  import AgentChat from '$lib/components/AgentChat.svelte';
  import type { AgentStatus } from '$lib/agent-types';

  let status: AgentStatus = 'uninitialized';
  let agentInitialized = false;

  function handleStatusChange(event: CustomEvent<AgentStatus>) {
    status = event.detail;
  }

  function handleInitialized() {
    agentInitialized = true;
  }
</script>

<main>
  <h1>Tauri Rig Agent</h1>
  
  <AgentConfigPanel 
    {status} 
    on:statusChange={handleStatusChange} 
    on:initialized={handleInitialized} 
  />
  
  <AgentChat 
    bind:status 
    disabled={!agentInitialized} 
  />
</main>

<style>
  main {
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem;
    font-family: sans-serif;
  }
  h1 {
    text-align: center;
    margin-bottom: 2rem;
  }
</style>