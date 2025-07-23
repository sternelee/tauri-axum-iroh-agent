<script lang="ts">
	import { onMount, afterUpdate } from 'svelte';
	import { chatStore } from '$lib/stores/chatStore';
	import MessageBubble from './MessageBubble.svelte';
	import type { ChatMessage } from '$lib/types/chat';

	let messagesContainer: HTMLDivElement;
	let shouldAutoScroll = true;

	// 订阅聊天状态
	$: ({ messages, isLoading } = $chatStore);

	// 自动滚动到底部
	function scrollToBottom() {
		if (messagesContainer && shouldAutoScroll) {
			messagesContainer.scrollTop = messagesContainer.scrollHeight;
		}
	}

	// 检查是否应该自动滚动
	function handleScroll() {
		if (messagesContainer) {
			const { scrollTop, scrollHeight, clientHeight } = messagesContainer;
			// 如果用户滚动到接近底部（50px内），则启用自动滚动
			shouldAutoScroll = scrollTop + clientHeight >= scrollHeight - 50;
		}
	}

	// 组件挂载后滚动到底部
	onMount(() => {
		scrollToBottom();
	});

	// 每次更新后检查是否需要滚动
	afterUpdate(() => {
		scrollToBottom();
	});
</script>

<div class="message-list-container">
	<!-- 消息列表头部 -->
	<div class="message-list-header">
		<div class="header-content">
			<div class="chat-title">
				<h1>ChatGPT 助手</h1>
				<p>智能对话助手，随时为您提供帮助</p>
			</div>
			<div class="chat-status">
				{#if isLoading}
					<div class="status-indicator loading">
						<div class="loading-dot"></div>
						<span>AI正在思考...</span>
					</div>
				{:else}
					<div class="status-indicator online">
						<div class="online-dot"></div>
						<span>在线</span>
					</div>
				{/if}
			</div>
		</div>
	</div>

	<!-- 消息滚动区域 -->
	<div 
		class="messages-scroll-area" 
		bind:this={messagesContainer}
		on:scroll={handleScroll}
	>
		<div class="messages-content">
			<!-- 欢迎消息 -->
			{#if messages.length === 1}
				<div class="welcome-section">
					<div class="welcome-icon">
						<svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
						</svg>
					</div>
					<h2>欢迎使用 ChatGPT 助手</h2>
					<p>我可以帮助您解答问题、提供建议或进行有趣的对话。请随时向我提问！</p>
				</div>
			{/if}

			<!-- 消息列表 -->
			<div class="messages-list">
				{#each messages as message (message.id)}
					<MessageBubble {message} />
				{/each}

				<!-- 加载指示器 -->
				{#if isLoading}
					<MessageBubble 
						message={{
							id: 'typing',
							content: '',
							sender: 'ai',
							timestamp: new Date()
						}}
						isTyping={true}
					/>
				{/if}
			</div>
		</div>
	</div>

	<!-- 滚动到底部按钮 -->
	{#if !shouldAutoScroll}
		<button 
			class="scroll-to-bottom"
			on:click={scrollToBottom}
			title="滚动到底部"
		>
			<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<path d="M7 13l3 3 7-7"/>
				<path d="M7 6l3 3 7-7"/>
			</svg>
		</button>
	{/if}
</div>

<style>
	.message-list-container {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: #f7f7f8;
		position: relative;
	}

	/* 头部样式 */
	.message-list-header {
		background: white;
		border-bottom: 1px solid #e5e7eb;
		padding: 16px 24px;
		flex-shrink: 0;
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: center;
		max-width: 800px;
		margin: 0 auto;
	}

	.chat-title h1 {
		margin: 0;
		font-size: 20px;
		font-weight: 600;
		color: #111827;
	}

	.chat-title p {
		margin: 4px 0 0 0;
		font-size: 14px;
		color: #6b7280;
	}

	.status-indicator {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 14px;
		font-weight: 500;
	}

	.status-indicator.online {
		color: #059669;
	}

	.status-indicator.loading {
		color: #f59e0b;
	}

	.online-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #10b981;
	}

	.loading-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #f59e0b;
		animation: pulse 2s infinite;
	}

	/* 消息滚动区域 */
	.messages-scroll-area {
		flex: 1;
		overflow-y: auto;
		scroll-behavior: smooth;
	}

	.messages-content {
		max-width: 800px;
		margin: 0 auto;
		padding: 24px;
		min-height: 100%;
		display: flex;
		flex-direction: column;
	}

	/* 欢迎区域 */
	.welcome-section {
		text-align: center;
		padding: 40px 20px;
		color: #6b7280;
	}

	.welcome-icon {
		margin-bottom: 16px;
		color: #10b981;
	}

	.welcome-section h2 {
		margin: 0 0 12px 0;
		font-size: 24px;
		font-weight: 600;
		color: #111827;
	}

	.welcome-section p {
		margin: 0;
		font-size: 16px;
		line-height: 1.6;
		max-width: 500px;
		margin: 0 auto;
	}

	/* 消息列表 */
	.messages-list {
		flex: 1;
		padding-top: 20px;
	}

	/* 滚动到底部按钮 */
	.scroll-to-bottom {
		position: absolute;
		bottom: 100px;
		right: 24px;
		width: 44px;
		height: 44px;
		border-radius: 50%;
		background: white;
		border: 1px solid #e5e7eb;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition: all 0.2s ease;
		color: #6b7280;
	}

	.scroll-to-bottom:hover {
		background: #f9fafb;
		transform: translateY(-2px);
		box-shadow: 0 6px 16px rgba(0, 0, 0, 0.2);
	}

	/* 动画 */
	@keyframes pulse {
		0%, 100% {
			opacity: 1;
		}
		50% {
			opacity: 0.5;
		}
	}

	/* 响应式设计 */
	@media (max-width: 768px) {
		.message-list-header {
			padding: 12px 16px;
		}

		.header-content {
			flex-direction: column;
			align-items: flex-start;
			gap: 8px;
		}

		.messages-content {
			padding: 16px;
		}

		.welcome-section {
			padding: 24px 16px;
		}

		.welcome-section h2 {
			font-size: 20px;
		}

		.scroll-to-bottom {
			right: 16px;
			bottom: 80px;
		}
	}

	/* 滚动条样式 */
	.messages-scroll-area::-webkit-scrollbar {
		width: 6px;
	}

	.messages-scroll-area::-webkit-scrollbar-track {
		background: transparent;
	}

	.messages-scroll-area::-webkit-scrollbar-thumb {
		background: #d1d5db;
		border-radius: 3px;
	}

	.messages-scroll-area::-webkit-scrollbar-thumb:hover {
		background: #9ca3af;
	}
</style>