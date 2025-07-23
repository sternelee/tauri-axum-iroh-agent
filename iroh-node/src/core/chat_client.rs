//! iroh P2P聊天客户端实现

use super::chat::{
    ChatConfig, ChatEvent, ChatMessage, ChatRoom, ChatUser, CreateRoomRequest, JoinRoomRequest,
    LeaveRoomRequest, MessageType, SendMessageRequest,
};
use crate::core::error::{IrohTransferError, TransferResult};
use futures_lite::stream::StreamExt;
use iroh::client::Iroh;
use iroh_gossip::proto::TopicId;
use serde_json;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// 聊天事件回调函数类型
pub type ChatEventCallback = Box<dyn Fn(ChatEvent) + Send + Sync>;

/// iroh P2P聊天客户端
pub struct IrohChatClient {
    /// iroh客户端
    iroh_client: Iroh,
    /// 用户配置
    config: ChatConfig,
    /// 当前用户信息
    current_user: ChatUser,
    /// 已加入的聊天室
    joined_rooms: Arc<Mutex<HashMap<String, ChatRoom>>>,
    /// 消息历史
    message_history: Arc<Mutex<HashMap<String, Vec<ChatMessage>>>>,
    /// 事件广播器
    event_sender: broadcast::Sender<ChatEvent>,
    /// 事件接收器
    _event_receiver: broadcast::Receiver<ChatEvent>,
}

impl IrohChatClient {
    /// 创建新的聊天客户端
    pub async fn new(iroh_client: Iroh, config: ChatConfig) -> TransferResult<Self> {
        let current_user = ChatUser::new(config.user_name.clone());
        let (event_sender, event_receiver) = broadcast::channel(1000);

        info!("创建聊天客户端，用户: {}", current_user.name);

        Ok(Self {
            iroh_client,
            config,
            current_user,
            joined_rooms: Arc::new(Mutex::new(HashMap::new())),
            message_history: Arc::new(Mutex::new(HashMap::new())),
            event_sender,
            _event_receiver: event_receiver,
        })
    }

    /// 获取事件接收器
    pub fn subscribe_events(&self) -> broadcast::Receiver<ChatEvent> {
        self.event_sender.subscribe()
    }

    /// 创建聊天室
    pub async fn create_room(&self, request: CreateRoomRequest) -> TransferResult<ChatRoom> {
        let room = ChatRoom::new(request.name, request.description);

        info!("创建聊天室: {} (ID: {})", room.name, room.id);

        // 加入自己创建的聊天室
        self.join_room_internal(room.clone()).await?;

        // 发送聊天室创建事件
        let _ = self.event_sender.send(ChatEvent::RoomCreated(room.clone()));

        Ok(room)
    }

    /// 加入聊天室
    pub async fn join_room(&self, request: JoinRoomRequest) -> TransferResult<()> {
        // 创建或获取聊天室信息
        let room = ChatRoom {
            id: request.room_id.clone(),
            name: format!("聊天室_{}", &request.room_id[..8]),
            description: None,
            created_at: chrono::Utc::now(),
            online_users: 1,
            topic_id: TopicId::from(request.room_id.as_bytes()),
        };

        self.join_room_internal(room).await
    }

    /// 内部加入聊天室方法
    async fn join_room_internal(&self, room: ChatRoom) -> TransferResult<()> {
        info!("加入聊天室: {} (ID: {})", room.name, room.id);

        // 订阅gossip主题
        let mut gossip_stream = self
            .iroh_client
            .gossip()
            .subscribe(room.topic_id)
            .await
            .map_err(|e| IrohTransferError::network(format!("订阅gossip主题失败: {}", e)))?;

        // 存储聊天室信息
        {
            let mut rooms = self.joined_rooms.lock().unwrap();
            rooms.insert(room.id.clone(), room.clone());
        }

        // 发送加入消息
        let join_message = ChatMessage::new_system(
            format!("{} 加入了聊天室", self.current_user.name),
            room.id.clone(),
        );
        self.send_message_internal(&join_message).await?;

        // 发送用户加入事件
        let _ = self
            .event_sender
            .send(ChatEvent::UserJoined(self.current_user.clone()));

        // 启动消息监听任务
        let room_id = room.id.clone();
        let event_sender = self.event_sender.clone();
        let message_history = self.message_history.clone();
        let max_history = self.config.max_message_history;

        tokio::spawn(async move {
            while let Some(event) = gossip_stream.next().await {
                match event {
                    Ok(gossip_event) => {
                        if let Ok(message) =
                            serde_json::from_slice::<ChatMessage>(&gossip_event.content)
                        {
                            debug!("收到消息: {:?}", message);

                            // 存储消息历史
                            {
                                let mut history = message_history.lock().unwrap();
                                let room_messages =
                                    history.entry(room_id.clone()).or_insert_with(Vec::new);
                                room_messages.push(message.clone());

                                // 限制历史消息数量
                                if room_messages.len() > max_history {
                                    room_messages.remove(0);
                                }
                            }

                            // 发送消息接收事件
                            let _ = event_sender.send(ChatEvent::MessageReceived(message));
                        } else {
                            warn!("无法解析gossip消息");
                        }
                    }
                    Err(e) => {
                        error!("Gossip流错误: {}", e);
                        let _ = event_sender.send(ChatEvent::Error {
                            message: format!("网络连接错误: {}", e),
                        });
                    }
                }
            }
        });

        Ok(())
    }

    /// 发送消息
    pub async fn send_message(&self, request: SendMessageRequest) -> TransferResult<()> {
        let message = ChatMessage {
            id: Uuid::new_v4().to_string(),
            sender_id: self.current_user.id.clone(),
            sender_name: self.current_user.name.clone(),
            content: request.content,
            message_type: request.message_type,
            timestamp: chrono::Utc::now(),
            room_id: request.room_id,
        };

        self.send_message_internal(&message).await
    }

    /// 内部发送消息方法
    async fn send_message_internal(&self, message: &ChatMessage) -> TransferResult<()> {
        // 检查是否已加入聊天室
        let room = {
            let rooms = self.joined_rooms.lock().unwrap();
            rooms.get(&message.room_id).cloned()
        };

        let room = room.ok_or_else(|| {
            IrohTransferError::other(format!("未加入聊天室: {}", message.room_id))
        })?;

        // 序列化消息
        let message_data = serde_json::to_vec(message)
            .map_err(|e| IrohTransferError::other(format!("序列化消息失败: {}", e)))?;

        // 通过gossip发送消息
        self.iroh_client
            .gossip()
            .broadcast(room.topic_id, message_data.into())
            .await
            .map_err(|e| IrohTransferError::network(format!("发送gossip消息失败: {}", e)))?;

        debug!("发送消息: {:?}", message);
        Ok(())
    }

    /// 离开聊天室
    pub async fn leave_room(&self, request: LeaveRoomRequest) -> TransferResult<()> {
        info!("离开聊天室: {}", request.room_id);

        // 发送离开消息
        let leave_message = ChatMessage::new_system(
            format!("{} 离开了聊天室", self.current_user.name),
            request.room_id.clone(),
        );
        let _ = self.send_message_internal(&leave_message).await;

        // 从已加入的聊天室中移除
        {
            let mut rooms = self.joined_rooms.lock().unwrap();
            rooms.remove(&request.room_id);
        }

        // 发送用户离开事件
        let _ = self.event_sender.send(ChatEvent::UserLeft {
            user_id: self.current_user.id.clone(),
            user_name: self.current_user.name.clone(),
        });

        Ok(())
    }

    /// 分享文件到聊天室
    pub async fn share_file(
        &self,
        room_id: String,
        file_name: String,
        doc_ticket: String,
    ) -> TransferResult<()> {
        if !self.config.enable_file_sharing {
            return Err(IrohTransferError::other("文件分享功能已禁用"));
        }

        let message = ChatMessage::new_file_share(
            self.current_user.id.clone(),
            self.current_user.name.clone(),
            file_name,
            doc_ticket,
            room_id,
        );

        self.send_message_internal(&message).await
    }

    /// 获取聊天室列表
    pub fn get_joined_rooms(&self) -> Vec<ChatRoom> {
        let rooms = self.joined_rooms.lock().unwrap();
        rooms.values().cloned().collect()
    }

    /// 获取聊天室消息历史
    pub fn get_message_history(&self, room_id: &str) -> Vec<ChatMessage> {
        let history = self.message_history.lock().unwrap();
        history.get(room_id).cloned().unwrap_or_default()
    }

    /// 获取当前用户信息
    pub fn get_current_user(&self) -> &ChatUser {
        &self.current_user
    }

    /// 更新用户昵称
    pub fn update_user_name(&mut self, new_name: String) {
        self.current_user.name = new_name.clone();
        self.config.user_name = new_name;
        info!("更新用户昵称: {}", self.current_user.name);
    }
}
