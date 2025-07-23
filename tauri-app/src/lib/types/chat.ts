// 聊天消息类型定义
export interface ChatMessage {
  id: string;
  content: string;
  sender: 'user' | 'ai';
  timestamp: Date;
  isTyping?: boolean;
}

// 聊天状态类型
export interface ChatState {
  messages: ChatMessage[];
  isLoading: boolean;
  currentInput: string;
}

// 消息发送器类型
export type MessageSender = (content: string) => Promise<void>;