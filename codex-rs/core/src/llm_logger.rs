//! LLM 通信日志记录模块
//!
//! 仅在 debug 编译模式下启用，用于记录所有与 LLM 的通信日志。
//! 日志文件格式：JSON Lines (每行一个 JSON 对象)
//! 文件路径：~/.codeACE/debug_logs/llm_YYYY-MM-DD.jsonl

#![cfg(debug_assertions)]

use chrono::{DateTime, Local, Utc};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tracing::{debug, error};

/// LLM 日志条目
#[derive(Debug, Serialize)]
pub struct LlmLogEntry {
    /// ISO 8601 格式的时间戳
    pub timestamp: String,
    /// 日志类型：request 或 response
    #[serde(rename = "type")]
    pub log_type: String,
    /// API 类型：responses_api 或 chat_completions
    pub api: String,
    /// 请求 ID（如果有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// 原始数据
    pub data: Value,
}

/// LLM 日志记录器
pub struct LlmLogger {
    base_dir: PathBuf,
}

impl LlmLogger {
    /// 创建新的日志记录器实例
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let base_dir = home_dir.join(".codeACE").join("debug_logs");

        Self { base_dir }
    }

    /// 获取当前日期的日志文件路径
    fn get_log_file_path(&self) -> PathBuf {
        let now: DateTime<Local> = Local::now();
        let filename = format!("llm_{}.jsonl", now.format("%Y-%m-%d"));
        self.base_dir.join(filename)
    }

    /// 记录请求日志
    pub async fn log_request(&self, api: &str, request_id: Option<String>, data: Value) {
        self.write_log("request", api, request_id, data).await;
    }

    /// 记录响应日志
    pub async fn log_response(&self, api: &str, request_id: Option<String>, data: Value) {
        self.write_log("response", api, request_id, data).await;
    }

    /// 写入日志条目
    async fn write_log(&self, log_type: &str, api: &str, request_id: Option<String>, data: Value) {
        let entry = LlmLogEntry {
            timestamp: Utc::now().to_rfc3339(),
            log_type: log_type.to_string(),
            api: api.to_string(),
            request_id,
            data,
        };

        // 异步写入，错误不影响主流程
        if let Err(e) = self.write_entry(&entry).await {
            error!("Failed to write LLM log: {}", e);
        } else {
            debug!("LLM log written: {} {}", api, log_type);
        }
    }

    /// 写入单个日志条目到文件
    async fn write_entry(&self, entry: &LlmLogEntry) -> std::io::Result<()> {
        // 确保目录存在
        fs::create_dir_all(&self.base_dir).await?;

        // 获取当前日期的日志文件路径
        let log_path = self.get_log_file_path();

        // 序列化为 JSON 字符串
        let json_str = serde_json::to_string(entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // 追加写入文件
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .await?;

        file.write_all(json_str.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        Ok(())
    }
}

impl Default for LlmLogger {
    fn default() -> Self {
        Self::new()
    }
}

// 全局日志记录器单例
lazy_static::lazy_static! {
    static ref LOGGER: LlmLogger = LlmLogger::new();
}

/// 记录 Responses API 请求
pub async fn log_responses_request(request_id: Option<String>, data: Value) {
    LOGGER.log_request("responses_api", request_id, data).await;
}

/// 记录 Responses API 响应
pub async fn log_responses_response(request_id: Option<String>, data: Value) {
    LOGGER.log_response("responses_api", request_id, data).await;
}

/// 记录 Chat Completions API 请求
pub async fn log_chat_request(request_id: Option<String>, data: Value) {
    LOGGER
        .log_request("chat_completions", request_id, data)
        .await;
}

/// 记录 Chat Completions API 响应
pub async fn log_chat_response(request_id: Option<String>, data: Value) {
    LOGGER
        .log_response("chat_completions", request_id, data)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_llm_logger_basic() {
        let logger = LlmLogger::new();
        let test_data = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "test"}]
        });

        logger
            .log_request(
                "responses_api",
                Some("test-id-123".to_string()),
                test_data.clone(),
            )
            .await;

        logger
            .log_response("responses_api", Some("test-id-123".to_string()), test_data)
            .await;

        // 验证文件是否被创建
        let log_path = logger.get_log_file_path();
        assert!(log_path.exists());
    }

    #[tokio::test]
    async fn test_global_logger() {
        let test_data = json!({"test": "data"});
        log_responses_request(Some("global-test".to_string()), test_data.clone()).await;
        log_responses_response(Some("global-test".to_string()), test_data).await;
    }
}
