//! 错误处理模块

use std::fmt;
use thiserror::Error;

/// iroh传输错误类型
#[derive(Error, Debug)]
pub enum IrohTransferError {
    /// iroh客户端错误
    #[error("iroh客户端错误: {0}")]
    IrohClient(#[from] iroh::client::RpcError),

    /// 文档操作错误
    #[error("文档操作错误: {0}")]
    DocError(#[from] iroh::docs::store::fs::Error),

    /// IO错误
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    /// 票据解析错误
    #[error("票据解析错误: {0}")]
    TicketParse(String),

    /// 文件不存在
    #[error("文件不存在: {0}")]
    FileNotFound(String),

    /// 重复文件名
    #[error("重复文件名: {0}")]
    DuplicateFileName(String),

    /// 下载目录不存在
    #[error("下载目录不存在")]
    DownloadDirNotFound,

    /// 配置错误
    #[error("配置错误: {0}")]
    Config(String),

    /// 网络错误
    #[error("网络错误: {0}")]
    Network(String),

    /// 其他错误
    #[error("其他错误: {0}")]
    Other(String),
}

impl IrohTransferError {
    /// 创建票据解析错误
    pub fn ticket_parse<T: fmt::Display>(msg: T) -> Self {
        Self::TicketParse(msg.to_string())
    }

    /// 创建文件不存在错误
    pub fn file_not_found<T: fmt::Display>(path: T) -> Self {
        Self::FileNotFound(path.to_string())
    }

    /// 创建重复文件名错误
    pub fn duplicate_file_name<T: fmt::Display>(name: T) -> Self {
        Self::DuplicateFileName(name.to_string())
    }

    /// 创建配置错误
    pub fn config<T: fmt::Display>(msg: T) -> Self {
        Self::Config(msg.to_string())
    }

    /// 创建网络错误
    pub fn network<T: fmt::Display>(msg: T) -> Self {
        Self::Network(msg.to_string())
    }

    /// 创建其他错误
    pub fn other<T: fmt::Display>(msg: T) -> Self {
        Self::Other(msg.to_string())
    }
}

/// 传输结果类型别名
pub type TransferResult<T> = Result<T, IrohTransferError>;

/// 将anyhow::Error转换为IrohTransferError
impl From<anyhow::Error> for IrohTransferError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err.to_string())
    }
}
