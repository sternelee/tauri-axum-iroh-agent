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
		background: linear-gradient(135deg, #f0f4f8 0%, #e6f7ff 100%);
		display: flex;
		flex-direction: column;
		overflow: hidden;
		font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
	}

	.chat-container {
		height: 100%;
		display: flex;
		flex-direction: column;
		max-width: 100%;
		margin: 0 auto;
		background: white;
		box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.1), 0 8px 10px -6px rgba(0, 0, 0, 0.1);
		border-radius: 16px;
		overflow: hidden;
	}

	/* 响应式设计 */
	@media (min-width: 768px) {
		.chat-app {
			padding: 24px;
			background: linear-gradient(135deg, #f0f4f8 0%, #e6f7ff 100%);
		}

		.chat-container {
			border-radius: 16px;
			max-height: calc(100vh - 48px);
			margin: 0 auto;
		}
	}

	@media (max-width: 767px) {
		.chat-app {
			padding: 0;
			background: white;
		}

		.chat-container {
			border-radius: 0;
			height: 100vh;
			max-height: 100vh;
		}
	}
</style>
