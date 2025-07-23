//! 集成的iroh P2P客户端，同时支持文件传输和聊天功能

use super::{
    chat::{
        ChatConfig, ChatEvent, ChatMessage, ChatRoom, CreateRoomRequest, JoinRoomRequest,
        LeaveRoomRequest, SendMessageRequest,
    },
    chat_client::IrohChatClient,
    client::IrohClient,
    error::{IrohTransferError, TransferResult},
    progress::{ProgressNotifier, TransferEvent},
    types::{DownloadRequest, RemoveRequest, ShareResponse, TransferConfig, UploadRequest},
};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};

/// 集成的iroh客户端，同时支持文件传输和聊天
pub struct IrohIntegratedClient {
    /// 文件传输客户端
    transfer_client: Arc<IrohClient>,
    /// 聊天客户端
    chat_client: Option<Arc<IrohChatClient>>,
    /// 传输配置
    transfer_config: TransferConfig,
    /// 聊天配置
    chat_config: ChatConfig,
}

impl IrohIntegratedClient {
    /// 创建新的集成客户端
    pub async fn new(
        transfer_config: TransferConfig,
        chat_config: ChatConfig,
    ) -> TransferResult<Self> {
        let transfer_client = Arc::new(IrohClient::new(transfer_config.clone()).await?);

        info!("创建集成iroh客户端");

        Ok(Self {
            transfer_client,
            chat_client: None,
            transfer_config,
            chat_config,
        })
    }

    /// 启用聊天功能
    pub async fn enable_chat(&mut self) -> TransferResult<()> {
        if self.chat_client.is_some() {
            warn!("聊天功能已启用");
            return Ok(());
        }

        // 获取iroh客户端
        let iroh_client = self.transfer_client.client();

        // 创建聊天客户端
        let chat_client =
            Arc::new(IrohChatClient::new(iroh_client, self.chat_config.clone()).await?);
        self.chat_client = Some(chat_client);

        info!("聊天功能已启用");
        Ok(())
    }

    /// 禁用聊天功能
    pub async fn disable_chat(&mut self) -> TransferResult<()> {
        if let Some(chat_client) = self.chat_client.take() {
            // 优雅地关闭聊天客户端
            // 离开所有已加入的聊天室
            let rooms = chat_client.get_joined_rooms();
            for room in rooms {
                let _ = chat_client
                    .leave_room(LeaveRoomRequest { room_id: room.id })
                    .await;
            }
            info!("聊天功能已禁用，已离开所有聊天室");
        }
        Ok(())
    }

    /// 检查聊天功能是否启用
    pub fn is_chat_enabled(&self) -> bool {
        self.chat_client.is_some()
    }

    // === 文件传输功能 ===

    /// 上传文件
    pub async fn upload_file<N: ProgressNotifier>(
        &self,
        request: UploadRequest,
        notifier: Arc<N>,
    ) -> TransferResult<()> {
        self.transfer_client.upload_file(request, notifier).await
    }

    /// 下载文件
    pub async fn download_files<N: ProgressNotifier>(
        &self,
        request: DownloadRequest,
        notifier: Arc<N>,
    ) -> TransferResult<String> {
        self.transfer_client.download_files(request, notifier).await
    }

    /// 获取分享代码
    pub async fn get_share_code(&self) -> TransferResult<ShareResponse> {
        self.transfer_client.get_share_code().await
    }

    /// 删除文件
    pub async fn remove_file(&self, request: RemoveRequest) -> TransferResult<()> {
        self.transfer_client.remove_file(request).await
    }

    // === 聊天功能 ===

    /// 创建聊天室
    pub async fn create_chat_room(&self, request: CreateRoomRequest) -> TransferResult<ChatRoom> {
        let chat_client = self.get_chat_client()?;
        chat_client.create_room(request).await
    }

    /// 加入聊天室
    pub async fn join_chat_room(&self, request: JoinRoomRequest) -> TransferResult<()> {
        let chat_client = self.get_chat_client()?;
        chat_client.join_room(request).await
    }

    /// 发送聊天消息
    pub async fn send_chat_message(&self, request: SendMessageRequest) -> TransferResult<()> {
        let chat_client = self.get_chat_client()?;
        chat_client.send_message(request).await
    }

    /// 离开聊天室
    pub async fn leave_chat_room(&self, request: LeaveRoomRequest) -> TransferResult<()> {
        let chat_client = self.get_chat_client()?;
        chat_client.leave_room(request).await
    }

    /// 在聊天室中分享文件
    pub async fn share_file_in_chat(
        &self,
        room_id: String,
        file_name: String,
        doc_ticket: String,
    ) -> TransferResult<()> {
        let chat_client = self.get_chat_client()?;
        chat_client.share_file(room_id, file_name, doc_ticket).await
    }

    /// 订阅聊天事件
    pub fn subscribe_chat_events(&self) -> TransferResult<broadcast::Receiver<ChatEvent>> {
        let chat_client = self.get_chat_client()?;
        Ok(chat_client.subscribe_events())
    }

    /// 获取已加入的聊天室列表
    pub fn get_joined_rooms(&self) -> TransferResult<Vec<ChatRoom>> {
        let chat_client = self.get_chat_client()?;
        Ok(chat_client.get_joined_rooms())
    }

    /// 获取聊天室消息历史
    pub fn get_message_history(&self, room_id: &str) -> TransferResult<Vec<ChatMessage>> {
        let chat_client = self.get_chat_client()?;
        Ok(chat_client.get_message_history(room_id))
    }

    // === 集成功能 ===

    /// 上传文件并在聊天室中分享
    pub async fn upload_and_share_file<N: ProgressNotifier>(
        &self,
        upload_request: UploadRequest,
        room_id: String,
        notifier: Arc<N>,
    ) -> TransferResult<()> {
        // 先上传文件
        self.upload_file(upload_request.clone(), notifier).await?;

        // 获取分享代码
        let share_response = self.get_share_code().await?;

        // 获取文件名
        let file_name = upload_request
            .file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("未知文件")
            .to_string();

        // 在聊天室中分享
        self.share_file_in_chat(room_id, file_name, share_response.doc_ticket)
            .await?;

        Ok(())
    }

    /// 从聊天消息中下载文件
    pub async fn download_file_from_chat<N: ProgressNotifier>(
        &self,
        message: &ChatMessage,
        notifier: Arc<N>,
    ) -> TransferResult<String> {
        // 检查消息类型是否为文件分享
        if let super::chat::MessageType::FileShare { doc_ticket, .. } = &message.message_type {
            let download_request = DownloadRequest {
                doc_ticket: doc_ticket.clone(),
                download_dir: None,
            };

            self.download_files(download_request, notifier).await
        } else {
            Err(IrohTransferError::other("消息不是文件分享类型"))
        }
    }

    // === 私有辅助方法 ===

    /// 获取聊天客户端引用
    fn get_chat_client(&self) -> TransferResult<&Arc<IrohChatClient>> {
        self.chat_client
            .as_ref()
            .ok_or_else(|| IrohTransferError::other("聊天功能未启用，请先调用 enable_chat()"))
    }

    /// 获取底层传输客户端
    pub fn transfer_client(&self) -> &IrohClient {
        &self.transfer_client
    }

    /// 获取底层聊天客户端
    pub fn chat_client(&self) -> Option<&Arc<IrohChatClient>> {
        self.chat_client.as_ref()
    }

    /// 更新聊天用户昵称
    pub fn update_chat_user_name(&mut self, new_name: String) -> TransferResult<()> {
        self.chat_config.user_name = new_name.clone();

        if let Some(chat_client) = &self.chat_client {
            // 注意：这里需要修改IrohChatClient以支持可变引用
            // 暂时返回错误，提示需要重新启用聊天功能
            return Err(IrohTransferError::other(
                "要更改用户名，请先禁用聊天功能，更新配置后重新启用",
            ));
        }

        Ok(())
    }
}

/// 集成客户端构建器
pub struct IntegratedClientBuilder {
    transfer_config: TransferConfig,
    chat_config: ChatConfig,
    enable_chat: bool,
}

impl IntegratedClientBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            transfer_config: TransferConfig::default(),
            chat_config: ChatConfig::default(),
            enable_chat: false,
        }
    }

    /// 设置传输配置
    pub fn transfer_config(mut self, config: TransferConfig) -> Self {
        self.transfer_config = config;
        self
    }

    /// 设置聊天配置
    pub fn chat_config(mut self, config: ChatConfig) -> Self {
        self.chat_config = config;
        self
    }

    /// 启用聊天功能
    pub fn enable_chat(mut self, enable: bool) -> Self {
        self.enable_chat = enable;
        self
    }

    /// 构建集成客户端
    pub async fn build(self) -> TransferResult<IrohIntegratedClient> {
        let mut client = IrohIntegratedClient::new(self.transfer_config, self.chat_config).await?;

        if self.enable_chat {
            client.enable_chat().await?;
        }

        Ok(client)
    }
}

impl Default for IntegratedClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
