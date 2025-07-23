import { writable } from 'svelte/store';
import type { ChatMessage, ChatState } from '../types/chat';

// 生成AI回复的函数
function generateAIResponse(userMessage: string): string {
  const message = userMessage.toLowerCase();
  
  // 问候语回复
  if (message.includes('你好') || message.includes('hello') || message.includes('hi')) {
    const greetings = [
      '你好！很高兴与你对话。有什么我可以帮助你的吗？',
      '嗨！我是你的AI助手，随时准备为你提供帮助。',
      '你好！欢迎使用ChatGPT助手，我能为你做些什么？'
    ];
    return greetings[Math.floor(Math.random() * greetings.length)];
  }
  
  // 询问功能回复
  if (message.includes('你能做什么') || message.includes('功能') || message.includes('帮助')) {
    return '我可以帮助你：\n• 回答各种问题\n• 提供建议和解决方案\n• 进行有趣的对话\n• 协助学习和工作\n• 创意写作和头脑风暴\n\n请随时告诉我你需要什么帮助！';
  }
  
  // 技术相关回复
  if (message.includes('编程') || message.includes('代码') || message.includes('开发')) {
    const techResponses = [
      '编程是一门很有趣的技能！你想了解哪种编程语言或技术栈呢？',
      '我很乐意帮助你解决编程问题。请告诉我具体遇到了什么困难？',
      '代码开发确实需要不断学习和实践。你目前在学习什么技术？'
    ];
    return techResponses[Math.floor(Math.random() * techResponses.length)];
  }
  
  // 学习相关回复
  if (message.includes('学习') || message.includes('教') || message.includes('如何')) {
    const learningResponses = [
      '学习是一个持续的过程。你想学习什么具体内容呢？我可以为你制定学习计划。',
      '很好的学习态度！告诉我你的学习目标，我来帮你找到最佳的学习方法。',
      '我很乐意成为你的学习伙伴。请详细描述你想要学习的内容。'
    ];
    return learningResponses[Math.floor(Math.random() * learningResponses.length)];
  }
  
  // 感谢回复
  if (message.includes('谢谢') || message.includes('感谢') || message.includes('thanks')) {
    const thankResponses = [
      '不客气！很高兴能够帮助到你。还有其他问题吗？',
      '我很乐意为你提供帮助！如果还有什么需要，请随时告诉我。',
      '不用谢！这就是我存在的意义。有什么其他问题尽管问我。'
    ];
    return thankResponses[Math.floor(Math.random() * thankResponses.length)];
  }
  
  // 默认智能回复
  const defaultResponses = [
    '这是一个很有趣的话题！让我来详细为你分析一下。',
    '我理解你的观点。基于你提到的内容，我有以下几个想法：',
    '这确实是个值得深入思考的问题。从我的角度来看：',
    '你提出了一个很好的问题！让我从几个方面来回答：',
    '根据你的描述，我认为可以从以下角度来考虑这个问题：',
    '这个话题很有意思！我想分享一些相关的见解和建议：',
    '感谢你的分享！基于你提供的信息，我的建议是：'
  ];
  
  return defaultResponses[Math.floor(Math.random() * defaultResponses.length)];
}

// 创建聊天状态存储
function createChatStore() {
  const initialState: ChatState = {
    messages: [
      {
        id: '1',
        content: '你好！我是AI助手，有什么可以帮助你的吗？',
        sender: 'ai',
        timestamp: new Date()
      }
    ],
    isLoading: false,
    currentInput: ''
  };

  const { subscribe, set, update } = writable(initialState);

  return {
    subscribe,
    // 添加消息
    addMessage: (message: Omit<ChatMessage, 'id' | 'timestamp'>) => {
      update(state => ({
        ...state,
        messages: [
          ...state.messages,
          {
            ...message,
            id: Date.now().toString(),
            timestamp: new Date()
          }
        ]
      }));
    },
    // 设置加载状态
    setLoading: (loading: boolean) => {
      update(state => ({ ...state, isLoading: loading }));
    },
    // 更新输入内容
    updateInput: (input: string) => {
      update(state => ({ ...state, currentInput: input }));
    },
    // 清空输入
    clearInput: () => {
      update(state => ({ ...state, currentInput: '' }));
    },
    // 发送消息
    sendMessage: async (content: string) => {
      if (!content.trim()) return;
      
      // 添加用户消息
      update(state => ({
        ...state,
        messages: [
          ...state.messages,
          {
            id: Date.now().toString(),
            content: content.trim(),
            sender: 'user',
            timestamp: new Date()
          }
        ],
        currentInput: '',
        isLoading: true
      }));

      // 模拟AI回复（延迟1-3秒）
      setTimeout(() => {
        const aiResponse = generateAIResponse(content.trim());
        
        update(state => ({
          ...state,
          messages: [
            ...state.messages,
            {
              id: (Date.now() + 1).toString(),
              content: aiResponse,
              sender: 'ai',
              timestamp: new Date()
            }
          ],
          isLoading: false
        }));
      }, 1000 + Math.random() * 2000);
    }
  };
}

export const chatStore = createChatStore();