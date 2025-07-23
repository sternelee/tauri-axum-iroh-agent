<script lang="ts">
	import type { ChatMessage } from '$lib/types/chat';

	export let message: ChatMessage;
	export let isTyping: boolean = false;

	// 格式化时间显示
	function formatTime(date: Date): string {
		return date.toLocaleTimeString('zh-CN', {
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<div class="message-container" class:user={message.sender === 'user'} class:ai={message.sender === 'ai'}>
	<div class="message-wrapper">
		<!-- 头像 -->
		<div class="avatar">
			{#if message.sender === 'user'}
				<div class="user-avatar">
					<svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
						<path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
					</svg>
				</div>
			{:else}
				<div class="ai-avatar">
					<svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
						<path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"/>
					</svg>
				</div>
			{/if}
		</div>

		<!-- 消息气泡 -->
		<div class="message-bubble">
			<div class="message-content">
				{#if isTyping}
					<div class="typing-indicator">
						<span></span>
						<span></span>
						<span></span>
					</div>
				{:else}
					<p>{message.content}</p>
				{/if}
			</div>
			<div class="message-time">
				{formatTime(message.timestamp)}
			</div>
		</div>
	</div>
</div>

<style>
	.message-container {
		display: flex;
		margin-bottom: 16px;
		animation: fadeIn 0.3s ease-out;
	}

	.message-container.user {
		justify-content: flex-end;
	}

	.message-container.ai {
		justify-content: flex-start;
	}

	.message-wrapper {
		display: flex;
		align-items: flex-end;
		max-width: 70%;
		gap: 8px;
	}

	.user .message-wrapper {
		flex-direction: row-reverse;
	}

	.avatar {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.user-avatar {
		background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
		color: white;
	}

	.ai-avatar {
		background: linear-gradient(135deg, #10b981 0%, #059669 100%);
		color: white;
	}

	.message-bubble {
		border-radius: 18px;
		padding: 12px 16px;
		position: relative;
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
	}

	.user .message-bubble {
		background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
		color: white;
	}

	.ai .message-bubble {
		background: white;
		color: #374151;
		border: 1px solid #e5e7eb;
	}

	.message-content {
		margin-bottom: 4px;
	}

	.message-content p {
		margin: 0;
		line-height: 1.5;
		word-wrap: break-word;
	}

	.message-time {
		font-size: 11px;
		opacity: 0.7;
		text-align: right;
	}

	.user .message-time {
		color: rgba(255, 255, 255, 0.8);
	}

	.ai .message-time {
		color: #6b7280;
	}

	/* 打字指示器动画 */
	.typing-indicator {
		display: flex;
		gap: 4px;
		align-items: center;
	}

	.typing-indicator span {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background-color: #9ca3af;
		animation: typing 1.4s infinite ease-in-out;
	}

	.typing-indicator span:nth-child(1) {
		animation-delay: -0.32s;
	}

	.typing-indicator span:nth-child(2) {
		animation-delay: -0.16s;
	}

	@keyframes typing {
		0%, 80%, 100% {
			transform: scale(0.8);
			opacity: 0.5;
		}
		40% {
			transform: scale(1);
			opacity: 1;
		}
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
			transform: translateY(10px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	/* 响应式设计 */
	@media (max-width: 640px) {
		.message-wrapper {
			max-width: 85%;
		}
		
		.message-bubble {
			padding: 10px 14px;
		}
		
		.avatar {
			width: 28px;
			height: 28px;
		}
	}
</style>