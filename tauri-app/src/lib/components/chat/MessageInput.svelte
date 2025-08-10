<script lang="ts">
	import { chatStore } from '$lib/stores/chatStore';
	import { Button } from '$lib/components/ui/button/index';

	let textareaElement: HTMLTextAreaElement;
	let inputValue = '';
	let isComposing = false; // 处理中文输入法

	// 订阅聊天状态
	$: ({ isLoading } = $chatStore);

	// 自动调整文本框高度
	function autoResize() {
		if (textareaElement) {
			textareaElement.style.height = 'auto';
			textareaElement.style.height = Math.min(textareaElement.scrollHeight, 150) + 'px';
		}
	}

	// 发送消息
	async function sendMessage() {
		const message = inputValue.trim();
		if (!message || isLoading) return;

		// 清空输入框
		inputValue = '';
		autoResize();

		// 发送消息到store
		await chatStore.sendMessage(message);

		// 重新聚焦输入框
		textareaElement?.focus();
	}

	// 处理键盘事件
	function handleKeydown(event: KeyboardEvent) {
		// 中文输入法输入时不处理
		if (isComposing) return;

		if (event.key === 'Enter') {
			if (event.shiftKey) {
				// Shift + Enter 换行，不做处理
				return;
			} else {
				// Enter 发送消息
				event.preventDefault();
				sendMessage();
			}
		}
	}

	// 处理输入变化
	function handleInput() {
		autoResize();
	}

	// 处理中文输入法
	function handleCompositionStart() {
		isComposing = true;
	}

	function handleCompositionEnd() {
		isComposing = false;
	}

	// 粘贴处理
	function handlePaste(event: ClipboardEvent) {
		// 延迟执行以确保内容已粘贴
		setTimeout(() => {
			autoResize();
		}, 0);
	}
</script>

<div class="message-input-container">
	<div class="input-wrapper">
		<!-- 输入区域 -->
		<div class="input-area">
			<textarea
				bind:this={textareaElement}
				bind:value={inputValue}
				placeholder="输入消息... (Enter发送，Shift+Enter换行)"
				class="message-textarea"
				disabled={isLoading}
				on:keydown={handleKeydown}
				on:input={handleInput}
				on:compositionstart={handleCompositionStart}
				on:compositionend={handleCompositionEnd}
				on:paste={handlePaste}
				rows="1"
			></textarea>

			<!-- 发送按钮 -->
			<Button
				class="send-button"
				disabled={!inputValue.trim() || isLoading}
				on:click={sendMessage}
				size="sm"
			>
				{#if isLoading}
					<div class="loading-spinner">
						<svg width="16" height="16" viewBox="0 0 24 24" fill="none">
							<circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2" opacity="0.25"/>
							<path d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" fill="currentColor"/>
						</svg>
					</div>
				{:else}
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<line x1="22" y1="2" x2="11" y2="13"></line>
						<polygon points="22,2 15,22 11,13 2,9"></polygon>
					</svg>
				{/if}
			</Button>
		</div>

		<!-- 提示信息 -->
		<div class="input-hints">
			<div class="hint-left">
				{#if isLoading}
					<span class="hint-loading">
						<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M21 12a9 9 0 1 1-6.219-8.56"></path>
						</svg>
						AI正在思考中...
					</span>
				{:else}
					<span class="hint-normal">支持 Markdown 格式</span>
				{/if}
			</div>
			<div class="hint-right">
				<span class="hint-shortcut">Enter 发送 • Shift+Enter 换行</span>
			</div>
		</div>
	</div>
</div>

<style>
	.message-input-container {
		background: linear-gradient(to top, #ffffff 0%, #f8fafc 100%);
		border-top: 1px solid #e2e8f0;
		padding: 20px 24px 24px;
		flex-shrink: 0;
	}

	.input-wrapper {
		max-width: 800px;
		margin: 0 auto;
	}

	.input-area {
		display: flex;
		align-items: flex-end;
		gap: 12px;
		background: white;
		border: 1px solid #e2e8f0;
		border-radius: 16px;
		padding: 12px;
		transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
		position: relative;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
	}

	.input-area:focus-within {
		border-color: #0ea5e9;
		box-shadow: 0 0 0 4px rgba(14, 165, 233, 0.2);
		background: white;
	}

	.message-textarea {
		flex: 1;
		border: none;
		outline: none;
		background: transparent;
		resize: none;
		font-size: 15px;
		line-height: 1.6;
		color: #0f172a;
		font-family: inherit;
		min-height: 24px;
		max-height: 150px;
		overflow-y: auto;
		padding: 4px 0;
	}

	.message-textarea::placeholder {
		color: #94a3b8;
	}

	.message-textarea:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	/* 发送按钮样式 */
	:global(.send-button) {
		background: linear-gradient(135deg, #0ea5e9 0%, #0284c7 100%) !important;
		border: none !important;
		color: white !important;
		width: 40px !important;
		height: 40px !important;
		border-radius: 12px !important;
		padding: 0 !important;
		display: flex !important;
		align-items: center !important;
		justify-content: center !important;
		transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1) !important;
		flex-shrink: 0 !important;
		box-shadow: 0 2px 6px rgba(14, 165, 233, 0.3) !important;
	}

	:global(.send-button:hover:not(:disabled)) {
		transform: translateY(-2px) !important;
		box-shadow: 0 4px 12px rgba(14, 165, 233, 0.4) !important;
	}

	:global(.send-button:active:not(:disabled)) {
		transform: translateY(0) !important;
	}

	:global(.send-button:disabled) {
		opacity: 0.5 !important;
		cursor: not-allowed !important;
		transform: none !important;
		box-shadow: none !important;
	}

	.loading-spinner {
		animation: spin 1s linear infinite;
	}

	/* 提示信息 */
	.input-hints {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 12px;
		font-size: 13px;
		color: #64748b;
	}

	.hint-loading {
		color: #f59e0b;
		font-weight: 500;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.hint-loading svg {
		animation: spin 1.5s linear infinite;
	}

	.hint-normal {
		color: #64748b;
	}

	.hint-shortcut {
		color: #94a3b8;
		background: #f1f5f9;
		padding: 2px 8px;
		border-radius: 20px;
		font-size: 12px;
	}

	/* 动画 */
	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}

	/* 响应式设计 */
	@media (max-width: 768px) {
		.message-input-container {
			padding: 16px 16px 20px;
		}

		.input-area {
			padding: 10px;
			border-radius: 14px;
		}

		.input-hints {
			flex-direction: column;
			align-items: flex-start;
			gap: 6px;
			margin-top: 10px;
		}

		.hint-right {
			align-self: flex-end;
		}
		
		.message-textarea {
			font-size: 16px; /* 移动端增大字体 */
		}
	}

	/* 滚动条样式 */
	.message-textarea::-webkit-scrollbar {
		width: 6px;
	}

	.message-textarea::-webkit-scrollbar-track {
		background: transparent;
	}

	.message-textarea::-webkit-scrollbar-thumb {
		background: #cbd5e1;
		border-radius: 3px;
	}

	.message-textarea::-webkit-scrollbar-thumb:hover {
		background: #94a3b8;
	}
</style>