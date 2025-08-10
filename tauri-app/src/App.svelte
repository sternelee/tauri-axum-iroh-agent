<script lang="ts">
	import MessageList from '$lib/components/chat/MessageList.svelte';
	import MessageInput from '$lib/components/chat/MessageInput.svelte';
</script>

<main class="chat-app">
	<div class="chat-container">
		<MessageList />
		<MessageInput />
	</div>
</main>

<style>
	.chat-app {
		height: 100vh;
		width: 100vw;
		background: #f7f7f8;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.chat-container {
		height: 100%;
		display: flex;
		flex-direction: column;
		max-width: 100%;
		margin: 0 auto;
		background: white;
		box-shadow: 0 0 20px rgba(0, 0, 0, 0.1);
	}

	/* 响应式设计 */
	@media (min-width: 768px) {
		.chat-app {
			padding: 20px;
		}

		.chat-container {
			border-radius: 12px;
			max-height: calc(100vh - 40px);
		}
	}

	@media (max-width: 767px) {
		.chat-container {
			border-radius: 0;
		}
	}
</style>
