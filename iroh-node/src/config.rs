//! 节点配置

use iroh_net::relay::RelayUrl;
use serde::{Deserialize, Serialize};

/// 节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// 密钥
    pub secret_key: Option<String>,
    /// 中继服务器URL
    pub relay: Option<RelayUrl>,
    /// 禁用中继
    pub no_relay: bool,
    /// 节点名称
    pub name: Option<String>,
    /// 绑定端口
    pub bind_port: u16,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            secret_key: None,
            relay: None,
            no_relay: false,
            name: None,
            bind_port: 0, // 使用随机端口
        }
    }
}

impl NodeConfig {
    /// 创建新的配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置密钥
    pub fn with_secret_key(mut self, secret_key: Option<String>) -> Self {
        self.secret_key = secret_key;
        self
    }

    /// 设置中继服务器URL
    pub fn with_relay(mut self, relay: Option<RelayUrl>) -> Self {
        self.relay = relay;
        self
    }

    /// 设置是否禁用中继
    pub fn with_no_relay(mut self, no_relay: bool) -> Self {
        self.no_relay = no_relay;
        self
    }

    /// 设置节点名称
    pub fn with_name(mut self, name: Option<String>) -> Self {
        self.name = name;
        self
    }

    /// 设置绑定端口
    pub fn with_bind_port(mut self, bind_port: u16) -> Self {
        self.bind_port = bind_port;
        self
    }
}