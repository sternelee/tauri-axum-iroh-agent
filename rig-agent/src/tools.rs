//! Agent 工具模块

use crate::error::{AgentError, AgentResult};
use crate::core::types::{ToolCall, ToolResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::Utc;

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 参数定义（JSON Schema）
    pub parameters: serde_json::Value,
    /// 是否必需
    pub required: bool,
}

/// 内置工具集合
pub struct BuiltinTools {
    tools: HashMap<String, ToolDefinition>,
}

impl BuiltinTools {
    /// 创建内置工具集合
    pub fn new() -> Self {
        let mut tools = HashMap::new();
        
        // 添加计算器工具
        tools.insert("calculator".to_string(), ToolDefinition {
            name: "calculator".to_string(),
            description: "执行基本的数学计算".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "要计算的数学表达式，例如：2+3*4"
                    }
                },
                "required": ["expression"]
            }),
            required: false,
        });

        // 添加时间工具
        tools.insert("current_time".to_string(), ToolDefinition {
            name: "current_time".to_string(),
            description: "获取当前时间".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "timezone": {
                        "type": "string",
                        "description": "时区，例如：Asia/Shanghai",
                        "default": "UTC"
                    }
                }
            }),
            required: false,
        });

        // 添加天气工具（示例）
        tools.insert("weather".to_string(), ToolDefinition {
            name: "weather".to_string(),
            description: "获取指定城市的天气信息".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "city": {
                        "type": "string",
                        "description": "城市名称，例如：北京"
                    },
                    "unit": {
                        "type": "string",
                        "description": "温度单位：celsius 或 fahrenheit",
                        "default": "celsius"
                    }
                },
                "required": ["city"]
            }),
            required: false,
        });

        Self { tools }
    }

    /// 获取所有工具定义
    pub fn get_all_tools(&self) -> Vec<ToolDefinition> {
        self.tools.values().cloned().collect()
    }

    /// 获取指定工具定义
    pub fn get_tool(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    /// 执行工具
    pub async fn execute_tool(&self, tool_call: &ToolCall) -> AgentResult<ToolResult> {
        let start_time = std::time::Instant::now();
        
        let result = match tool_call.name.as_str() {
            "calculator" => self.execute_calculator(tool_call).await,
            "current_time" => self.execute_current_time(tool_call).await,
            "weather" => self.execute_weather(tool_call).await,
            _ => Err(AgentError::tool(format!("未知工具: {}", tool_call.name))),
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(result_content) => Ok(ToolResult {
                call_id: tool_call.id.clone(),
                tool_name: tool_call.name.clone(),
                result: result_content,
                success: true,
                error: None,
                timestamp: Utc::now(),
                duration_ms,
            }),
            Err(error) => Ok(ToolResult {
                call_id: tool_call.id.clone(),
                tool_name: tool_call.name.clone(),
                result: "".to_string(),
                success: false,
                error: Some(error.to_string()),
                timestamp: Utc::now(),
                duration_ms,
            }),
        }
    }

    /// 执行计算器工具
    async fn execute_calculator(&self, tool_call: &ToolCall) -> AgentResult<String> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.arguments)?;
        let expression = args["expression"]
            .as_str()
            .ok_or_else(|| AgentError::tool("缺少 expression 参数"))?;

        // 简单的数学表达式计算（实际应用中可以使用更强大的表达式解析器）
        let result = self.evaluate_expression(expression)?;
        Ok(format!("{} = {}", expression, result))
    }

    /// 执行当前时间工具
    async fn execute_current_time(&self, tool_call: &ToolCall) -> AgentResult<String> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.arguments)
            .unwrap_or_else(|_| serde_json::json!({}));
        
        let timezone = args["timezone"]
            .as_str()
            .unwrap_or("UTC");

        let now = Utc::now();
        let formatted_time = now.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        
        Ok(format!("当前时间（{}）: {}", timezone, formatted_time))
    }

    /// 执行天气工具（示例实现）
    async fn execute_weather(&self, tool_call: &ToolCall) -> AgentResult<String> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.arguments)?;
        let city = args["city"]
            .as_str()
            .ok_or_else(|| AgentError::tool("缺少 city 参数"))?;
        let unit = args["unit"].as_str().unwrap_or("celsius");

        // 这里是示例实现，实际应用中需要调用真实的天气API
        Ok(format!("{}的天气：晴朗，温度 25°{}", city, 
            if unit == "fahrenheit" { "F" } else { "C" }))
    }

    /// 简单的数学表达式计算
    fn evaluate_expression(&self, expression: &str) -> AgentResult<f64> {
        // 这是一个非常简单的实现，实际应用中应该使用专门的表达式解析器
        let cleaned = expression.replace(" ", "");
        
        // 支持基本的四则运算
        if let Some(pos) = cleaned.find('+') {
            let (left, right) = cleaned.split_at(pos);
            let right = &right[1..];
            let left_val = self.evaluate_expression(left)?;
            let right_val = self.evaluate_expression(right)?;
            return Ok(left_val + right_val);
        }
        
        if let Some(pos) = cleaned.rfind('-') {
            if pos > 0 {
                let (left, right) = cleaned.split_at(pos);
                let right = &right[1..];
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                return Ok(left_val - right_val);
            }
        }
        
        if let Some(pos) = cleaned.rfind('*') {
            let (left, right) = cleaned.split_at(pos);
            let right = &right[1..];
            let left_val = self.evaluate_expression(left)?;
            let right_val = self.evaluate_expression(right)?;
            return Ok(left_val * right_val);
        }
        
        if let Some(pos) = cleaned.rfind('/') {
            let (left, right) = cleaned.split_at(pos);
            let right = &right[1..];
            let left_val = self.evaluate_expression(left)?;
            let right_val = self.evaluate_expression(right)?;
            if right_val == 0.0 {
                return Err(AgentError::tool("除零错误"));
            }
            return Ok(left_val / right_val);
        }
        
        // 解析数字
        cleaned.parse::<f64>()
            .map_err(|_| AgentError::tool(format!("无法解析表达式: {}", expression)))
    }
}

impl Default for BuiltinTools {
    fn default() -> Self {
        Self::new()
    }
}

/// 自定义工具特征
pub trait CustomTool: Send + Sync {
    /// 工具名称
    fn name(&self) -> &str;
    
    /// 工具描述
    fn description(&self) -> &str;
    
    /// 参数定义
    fn parameters(&self) -> serde_json::Value;
    
    /// 执行工具
    async fn execute(&self, arguments: &str) -> AgentResult<String>;
}

/// 工具管理器
pub struct ToolManager {
    builtin_tools: BuiltinTools,
    custom_tools: HashMap<String, Box<dyn CustomTool>>,
}

impl ToolManager {
    /// 创建工具管理器
    pub fn new() -> Self {
        Self {
            builtin_tools: BuiltinTools::new(),
            custom_tools: HashMap::new(),
        }
    }

    /// 添加自定义工具
    pub fn add_custom_tool(&mut self, tool: Box<dyn CustomTool>) {
        let name = tool.name().to_string();
        self.custom_tools.insert(name, tool);
    }

    /// 移除自定义工具
    pub fn remove_custom_tool(&mut self, name: &str) -> bool {
        self.custom_tools.remove(name).is_some()
    }

    /// 获取所有工具定义
    pub fn get_all_tool_definitions(&self) -> Vec<ToolDefinition> {
        let mut tools = self.builtin_tools.get_all_tools();
        
        for custom_tool in self.custom_tools.values() {
            tools.push(ToolDefinition {
                name: custom_tool.name().to_string(),
                description: custom_tool.description().to_string(),
                parameters: custom_tool.parameters(),
                required: false,
            });
        }
        
        tools
    }

    /// 获取可用工具（用于 rig-core）
    pub fn get_available_tools(&self) -> Vec<rig_core::tool::Tool> {
        let mut tools = Vec::new();
        
        // 添加内置工具
        for tool_def in self.builtin_tools.get_all_tools() {
            if let Ok(tool) = rig_core::tool::Tool::new(
                tool_def.name,
                tool_def.description,
                tool_def.parameters,
            ) {
                tools.push(tool);
            }
        }
        
        // 添加自定义工具
        for custom_tool in self.custom_tools.values() {
            if let Ok(tool) = rig_core::tool::Tool::new(
                custom_tool.name().to_string(),
                custom_tool.description().to_string(),
                custom_tool.parameters(),
            ) {
                tools.push(tool);
            }
        }
        
        tools
    }

    /// 执行工具
    pub async fn execute_tool(&self, tool_call: &ToolCall) -> AgentResult<ToolResult> {
        // 先尝试内置工具
        if self.builtin_tools.get_tool(&tool_call.name).is_some() {
            return self.builtin_tools.execute_tool(tool_call).await;
        }

        // 再尝试自定义工具
        if let Some(custom_tool) = self.custom_tools.get(&tool_call.name) {
            let start_time = std::time::Instant::now();
            
            let result = custom_tool.execute(&tool_call.arguments).await;
            let duration_ms = start_time.elapsed().as_millis() as u64;

            return match result {
                Ok(result_content) => Ok(ToolResult {
                    call_id: tool_call.id.clone(),
                    tool_name: tool_call.name.clone(),
                    result: result_content,
                    success: true,
                    error: None,
                    timestamp: Utc::now(),
                    duration_ms,
                }),
                Err(error) => Ok(ToolResult {
                    call_id: tool_call.id.clone(),
                    tool_name: tool_call.name.clone(),
                    result: "".to_string(),
                    success: false,
                    error: Some(error.to_string()),
                    timestamp: Utc::now(),
                    duration_ms,
                }),
            };
        }

        Err(AgentError::tool(format!("未找到工具: {}", tool_call.name)))
    }

    /// 检查工具是否存在
    pub fn has_tool(&self, name: &str) -> bool {
        self.builtin_tools.get_tool(name).is_some() || self.custom_tools.contains_key(name)
    }
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_tools_creation() {
        let tools = BuiltinTools::new();
        let all_tools = tools.get_all_tools();
        assert!(!all_tools.is_empty());
        assert!(tools.get_tool("calculator").is_some());
        assert!(tools.get_tool("current_time").is_some());
    }

    #[tokio::test]
    async fn test_calculator_tool() {
        let tools = BuiltinTools::new();
        let tool_call = ToolCall {
            id: "test_call".to_string(),
            name: "calculator".to_string(),
            arguments: r#"{"expression": "2+3"}"#.to_string(),
            timestamp: Utc::now(),
        };

        let result = tools.execute_tool(&tool_call).await.unwrap();
        assert!(result.success);
        assert!(result.result.contains("5"));
    }

    #[tokio::test]
    async fn test_current_time_tool() {
        let tools = BuiltinTools::new();
        let tool_call = ToolCall {
            id: "test_call".to_string(),
            name: "current_time".to_string(),
            arguments: "{}".to_string(),
            timestamp: Utc::now(),
        };

        let result = tools.execute_tool(&tool_call).await.unwrap();
        assert!(result.success);
        assert!(result.result.contains("当前时间"));
    }

    #[test]
    fn test_expression_evaluation() {
        let tools = BuiltinTools::new();
        assert_eq!(tools.evaluate_expression("5").unwrap(), 5.0);
        assert_eq!(tools.evaluate_expression("2+3").unwrap(), 5.0);
        assert_eq!(tools.evaluate_expression("10-4").unwrap(), 6.0);
        assert_eq!(tools.evaluate_expression("3*4").unwrap(), 12.0);
        assert_eq!(tools.evaluate_expression("8/2").unwrap(), 4.0);
    }
}