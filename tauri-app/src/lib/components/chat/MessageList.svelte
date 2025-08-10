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
					<div class="welcome-features">
						<div class="feature-item">
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
								<polyline points="22,4 12,14.01 9,11.01"></polyline>
							</svg>
							<span>智能问答</span>
						</div>
						<div class="feature-item">
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
								<polyline points="22,4 12,14.01 9,11.01"></polyline>
							</svg>
							<span>创意写作</span>
						</div>
						<div class="feature-item">
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
								<polyline points="22,4 12,14.01 9,11.01"></polyline>
							</svg>
							<span>编程辅助</span>
						</div>
					</div>
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
		background: linear-gradient(to bottom, #f8fafc 0%, #f1f5f9 100%);
		position: relative;
	}

	/* 头部样式 */
	.message-list-header {
		background: linear-gradient(135deg, #ffffff 0%, #f8fafc 100%);
		border-bottom: 1px solid #e2e8f0;
		padding: 20px 24px;
		flex-shrink: 0;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
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
		font-size: 22px;
		font-weight: 700;
		color: #0f172a;
		background: linear-gradient(135deg, #0f172a 0%, #334155 100%);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.chat-title p {
		margin: 4px 0 0 0;
		font-size: 14px;
		color: #64748b;
		font-weight: 400;
	}

	.status-indicator {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 14px;
		font-weight: 500;
		padding: 6px 12px;
		border-radius: 20px;
	}

	.status-indicator.online {
		color: #059669;
		background: rgba(16, 185, 129, 0.1);
	}

	.status-indicator.loading {
		color: #f59e0b;
		background: rgba(245, 158, 11, 0.1);
	}

	.online-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #10b981;
		box-shadow: 0 0 6px rgba(16, 185, 129, 0.5);
	}

	.loading-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #f59e0b;
		animation: pulse 2s infinite;
		box-shadow: 0 0 6px rgba(245, 158, 11, 0.5);
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
		padding: 48px 24px;
		color: #64748b;
		border-radius: 16px;
		background: white;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.03);
		margin-bottom: 24px;
	}

	.welcome-icon {
		margin-bottom: 20px;
		color: #0ea5e9;
		background: rgba(14, 165, 233, 0.1);
		width: 64px;
		height: 64px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		margin-left: auto;
		margin-right: auto;
		padding: 12px;
	}

	.welcome-section h2 {
		margin: 0 0 12px 0;
		font-size: 26px;
		font-weight: 700;
		color: #0f172a;
	}

	.welcome-section p {
		margin: 0 0 24px 0;
		font-size: 16px;
		line-height: 1.6;
		max-width: 500px;
		margin-left: auto;
		margin-right: auto;
		color: #475569;
	}

	.welcome-features {
		display: flex;
		justify-content: center;
		gap: 24px;
		flex-wrap: wrap;
		margin-top: 20px;
	}

	.feature-item {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 14px;
		color: #0ea5e9;
		font-weight: 500;
	}

	.feature-item svg {
		color: #0ea5e9;
	}

	/* 消息列表 */
	.messages-list {
		flex: 1;
		padding-top: 12px;
	}

	/* 滚动到底部按钮 */
	.scroll-to-bottom {
		position: absolute;
		bottom: 100px;
		right: 24px;
		width: 48px;
		height: 48px;
		border-radius: 50%;
		background: white;
		border: 1px solid #e2e8f0;
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
		color: #64748b;
		z-index: 10;
	}

	.scroll-to-bottom:hover {
		background: #f1f5f9;
		transform: translateY(-3px) scale(1.05);
		box-shadow: 0 6px 20px rgba(0, 0, 0, 0.15);
		color: #0f172a;
	}

	.scroll-to-bottom:active {
		transform: translateY(-1px) scale(1.02);
	}

	/* 动画 */
	@keyframes pulse {
		0%, 100% {
			opacity: 1;
			transform: scale(1);
		}
		50% {
			opacity: 0.7;
			transform: scale(1.1);
		}
	}

	/* 响应式设计 */
	@media (max-width: 768px) {
		.message-list-header {
			padding: 16px;
		}

		.header-content {
			flex-direction: column;
			align-items: flex-start;
			gap: 12px;
		}

		.messages-content {
			padding: 16px;
		}

		.welcome-section {
			padding: 32px 16px;
		}

		.welcome-section h2 {
			font-size: 22px;
		}

		.welcome-features {
			gap: 16px;
		}

		.scroll-to-bottom {
			right: 16px;
			bottom: 80px;
		}
	}

	/* 滚动条样式 */
	.messages-scroll-area::-webkit-scrollbar {
		width: 8px;
	}

	.messages-scroll-area::-webkit-scrollbar-track {
		background: transparent;
	}

	.messages-scroll-area::-webkit-scrollbar-thumb {
		background: #cbd5e1;
		border-radius: 4px;
	}

	.messages-scroll-area::-webkit-scrollbar-thumb:hover {
		background: #94a3b8;
	}
</style>