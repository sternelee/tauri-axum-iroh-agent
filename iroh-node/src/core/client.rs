//! iroh P2P传输客户端核心实现

use crate::core::{
    error::{IrohTransferError, TransferResult},
    progress::{ProgressNotifier, TransferEvent},
    types::{
        DownloadRequest, FileInfo, IrohState, RemoveRequest, ShareResponse, TransferConfig,
        UploadRequest,
    },
};
use anyhow::Result;
use futures_lite::stream::StreamExt;
use iroh::{
    base::node_addr::AddrInfoOptions,
    blobs::{
        export::ExportProgress,
        store::{ExportFormat, ExportMode},
    },
    client::{
        Doc, MemIroh as Iroh,
        docs::{ImportProgress, ShareMode},
    },
    docs::{AuthorId, DocTicket, store::Query},
    util::fs,
};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};
use tracing::{error, info, trace};

type IrohNode = iroh::node::Node<iroh::blobs::store::fs::Store>;

/// iroh P2P传输客户端
pub struct IrohClient {
    node: IrohNode,
    state: IrohState,
    config: TransferConfig,
}

impl IrohClient {
    /// 创建新的iroh客户端实例
    pub async fn new(config: TransferConfig) -> TransferResult<Self> {
        // 创建iroh节点
        let node = iroh::node::Node::persistent(&config.data_root)
            .await
            .map_err(|e| IrohTransferError::other(format!("创建iroh节点失败: {}", e)))?
            .spawn()
            .await
            .map_err(|e| IrohTransferError::other(format!("启动iroh节点失败: {}", e)))?;

        // 创建作者和文档
        let current_author = node
            .client()
            .authors()
            .create()
            .await
            .map_err(|e| IrohTransferError::other(format!("创建作者失败: {}", e)))?;

        let current_doc = node
            .docs()
            .create()
            .await
            .map_err(|e| IrohTransferError::other(format!("创建文档失败: {}", e)))?;

        let state = IrohState::new(current_author, current_doc);

        info!("iroh客户端初始化完成，数据目录: {:?}", config.data_root);

        Ok(Self {
            node,
            state,
            config,
        })
    }

    /// 获取iroh客户端
    pub fn client(&self) -> Iroh {
        self.node.client().clone()
    }

    /// 获取当前文档
    pub fn doc(&self) -> Doc {
        self.state.doc.clone()
    }

    /// 获取当前作者ID
    pub fn author(&self) -> AuthorId {
        self.state.author
    }

    /// 下载文件
    pub async fn download_files<N: ProgressNotifier>(
        &self,
        request: DownloadRequest,
        notifier: Arc<N>,
    ) -> TransferResult<String> {
        let ticket = DocTicket::from_str(&request.doc_ticket)
            .map_err(|e| IrohTransferError::ticket_parse(e))?;

        let doc = self
            .client()
            .docs()
            .import(ticket.clone())
            .await
            .map_err(IrohTransferError::from)?;

        let download_folder = request
            .download_dir
            .or_else(|| self.config.download_dir.clone())
            .ok_or(IrohTransferError::DownloadDirNotFound)?;

        // 确保下载目录存在
        std::fs::create_dir_all(&download_folder)?;

        let mut entries = doc
            .get_many(Query::all())
            .await
            .map_err(IrohTransferError::from)?;

        while let Some(entry) = entries.next().await {
            let entry = entry.map_err(IrohTransferError::from)?;
            let mut name = String::from_utf8_lossy(entry.key()).to_string();

            // 处理文件名
            if name.len() >= 2 {
                name.remove(name.len() - 1);
            }

            let dest = download_folder.join(&name);

            info!(
                "开始下载文件: {}, 大小: {}, 目标路径: {:?}",
                name,
                entry.content_len(),
                dest
            );

            let exp_format = ExportFormat::Blob;
            let exp_mode = ExportMode::Copy;

            let mut stream = self
                .client()
                .blobs()
                .export(entry.content_hash(), dest.clone(), exp_format, exp_mode)
                .await
                .map_err(IrohTransferError::from)?;

            let file_id = dest.display().to_string();

            while let Some(result) = stream.next().await {
                match result {
                    Ok(progress) => match progress {
                        ExportProgress::Found {
                            id: _,
                            hash: _,
                            size,
                            outpath: _,
                            meta: _,
                        } => {
                            let event = TransferEvent::DownloadQueueAppend {
                                id: file_id.clone(),
                                size: size.value(),
                                name: name.clone(),
                            };
                            notifier.notify(event);
                        }
                        ExportProgress::Progress { id: _, offset } => {
                            let event = TransferEvent::DownloadProgress {
                                id: file_id.clone(),
                                offset,
                            };
                            notifier.notify(event);
                        }
                        ExportProgress::Done { id: _ } => {
                            let event = TransferEvent::DownloadDone {
                                id: file_id.clone(),
                            };
                            notifier.notify(event);
                            break;
                        }
                        ExportProgress::AllDone => {
                            break;
                        }
                        ExportProgress::Abort(e) => {
                            error!("下载中止: {}", e);
                            let event = TransferEvent::TransferError {
                                id: file_id.clone(),
                                error: e.to_string(),
                            };
                            notifier.notify(event);
                        }
                    },
                    Err(err) => {
                        error!("下载错误: {}", err);
                        let event = TransferEvent::TransferError {
                            id: file_id.clone(),
                            error: err.to_string(),
                        };
                        notifier.notify(event);
                    }
                }
            }
        }

        Ok(format!("文件已下载到: {}", download_folder.display()))
    }

    /// 获取分享代码
    pub async fn get_share_code(&self) -> TransferResult<ShareResponse> {
        let doc_ticket = self
            .doc()
            .share(ShareMode::Read, AddrInfoOptions::default())
            .await
            .map_err(|e| IrohTransferError::other(format!("创建分享票据失败: {}", e)))?;

        Ok(ShareResponse {
            doc_ticket: doc_ticket.to_string(),
        })
    }

    /// 上传文件
    pub async fn upload_file<N: ProgressNotifier>(
        &self,
        request: UploadRequest,
        notifier: Arc<N>,
    ) -> TransferResult<()> {
        self.import_file_to_iroh(&request.file_path, notifier).await
    }

    /// 删除文件
    pub async fn remove_file(&self, request: RemoveRequest) -> TransferResult<()> {
        let name = request
            .file_path
            .file_name()
            .ok_or_else(|| IrohTransferError::file_not_found("文件没有名称"))?
            .to_string_lossy()
            .to_string();

        let key = fs::path_to_key(name, None, None)
            .map_err(|e| IrohTransferError::other(format!("路径转换为键失败: {}", e)))?;

        let _amount_deleted = self
            .doc()
            .del(self.author(), key)
            .await
            .map_err(|e| IrohTransferError::other(format!("从iroh删除文件失败: {}", e)))?;

        Ok(())
    }

    /// 内部方法：导入文件到iroh
    async fn import_file_to_iroh<N: ProgressNotifier>(
        &self,
        path: &Path,
        notifier: Arc<N>,
    ) -> TransferResult<()> {
        let name = path
            .file_name()
            .ok_or_else(|| IrohTransferError::file_not_found("文件没有名称"))?
            .to_string_lossy()
            .to_string();

        let key = fs::path_to_key(name.clone(), None, None)
            .map_err(|e| IrohTransferError::other(format!("路径转换为键失败: {}", e)))?;

        // 检查是否已存在同名文件
        let possible_entry = self
            .doc()
            .get_exact(self.author(), key.clone(), false)
            .await
            .map_err(IrohTransferError::from)?;

        if possible_entry.is_some() {
            return Err(IrohTransferError::duplicate_file_name(&name));
        }

        let mut stream = self
            .doc()
            .import_file(self.author(), key, path, true)
            .await
            .map_err(|e| IrohTransferError::other(format!("导入文件失败 \"{:?}\": {}", path, e)))?;

        let file_id = path.display().to_string();

        while let Some(result) = stream.next().await {
            match result {
                Ok(progress) => match progress {
                    ImportProgress::Found { id: _, name, size } => {
                        let event = TransferEvent::UploadQueueAppend {
                            id: file_id.clone(),
                            size,
                            title: name.clone(),
                        };
                        notifier.notify(event);
                    }
                    ImportProgress::Progress { id: _, offset } => {
                        let event = TransferEvent::UploadProgress {
                            id: file_id.clone(),
                            offset,
                        };
                        notifier.notify(event);
                    }
                    ImportProgress::IngestDone { id: _, hash: _ } => {
                        let event = TransferEvent::UploadDone {
                            id: file_id.clone(),
                        };
                        notifier.notify(event);
                    }
                    ImportProgress::AllDone { key: _ } => {}
                    ImportProgress::Abort(e) => {
                        error!("上传中止: {:?}", e);
                        let event = TransferEvent::TransferError {
                            id: file_id.clone(),
                            error: e.to_string(),
                        };
                        notifier.notify(event);
                    }
                },
                Err(err) => {
                    error!("上传错误: {}", err);
                    let event = TransferEvent::TransferError {
                        id: file_id.clone(),
                        error: err.to_string(),
                    };
                    notifier.notify(event);
                }
            }
        }

        Ok(())
    }
}
