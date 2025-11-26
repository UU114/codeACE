//! LLM 通信日志记录模块
//!
//! 仅在 debug 编译模式下启用，用于记录所有与 LLM 的通信日志。
//! 日志文件格式：JSON Lines (每行一个 JSON 对象)
//! 文件路径：~/.codeACE/debug_logs/llm_YYYY-MM-DD.jsonl

#![cfg(debug_assertions)]

use chrono::{DateTime, Local, Utc};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
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

/// 正在合并的响应数据
#[derive(Debug, Clone)]
struct MergingResponse {
    /// 第一个chunk的时间戳
    first_timestamp: String,
    /// 基础响应数据（包含id, created, model等公共字段）
    base_data: Value,
    /// 累积的assistant内容
    accumulated_content: String,
    /// 累积的reasoning内容
    accumulated_reasoning: String,
    /// 是否已完成（收到finish_reason）
    is_complete: bool,
}

/// LLM 日志记录器
pub struct LlmLogger {
    base_dir: PathBuf,
    /// 正在合并的chat completions响应（key: response_id）
    merging_responses: Arc<RwLock<HashMap<String, MergingResponse>>>,
}

impl LlmLogger {
    /// 创建新的日志记录器实例
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let base_dir = home_dir.join(".codeACE").join("debug_logs");

        Self {
            base_dir,
            merging_responses: Arc::new(RwLock::new(HashMap::new())),
        }
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

    /// 记录并合并 Chat Completions API 的流式响应
    ///
    /// 此方法会将同一个response_id的多个chunk合并成一条完整的日志记录
    pub async fn log_chat_response_merged(&self, data: Value) {
        // 提取response_id
        let response_id = match data.get("id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => {
                // 没有id字段，直接记录原始数据
                self.log_response("chat_completions", None, data).await;
                return;
            }
        };

        // 提取delta内容
        let mut content_delta = String::new();
        let mut reasoning_delta = String::new();
        let mut has_finish_reason = false;

        if let Some(choices) = data.get("choices").and_then(|v| v.as_array()) {
            if let Some(choice) = choices.first() {
                // 检查是否有finish_reason
                if choice.get("finish_reason").is_some() {
                    has_finish_reason = true;
                }

                // 提取delta中的content
                if let Some(content) = choice
                    .get("delta")
                    .and_then(|d| d.get("content"))
                    .and_then(|c| c.as_str())
                {
                    content_delta = content.to_string();
                }

                // 提取delta中的reasoning_content
                if let Some(reasoning) = choice
                    .get("delta")
                    .and_then(|d| d.get("reasoning_content"))
                    .and_then(|r| r.as_str())
                {
                    reasoning_delta = reasoning.to_string();
                }
            }
        }

        let mut responses = self.merging_responses.write().await;

        // 获取或创建MergingResponse
        let merging = responses
            .entry(response_id.clone())
            .or_insert_with(|| MergingResponse {
                first_timestamp: Utc::now().to_rfc3339(),
                base_data: data.clone(),
                accumulated_content: String::new(),
                accumulated_reasoning: String::new(),
                is_complete: false,
            });

        // 累积内容
        merging.accumulated_content.push_str(&content_delta);
        merging.accumulated_reasoning.push_str(&reasoning_delta);

        // 如果收到finish_reason，标记为完成
        if has_finish_reason {
            merging.is_complete = true;
        }

        // 如果已完成，构建合并后的日志并写入
        if merging.is_complete {
            let merged_data = self.build_merged_response(merging);
            let timestamp = merging.first_timestamp.clone();

            // 移除已完成的response
            responses.remove(&response_id);

            // 释放锁后再写入文件
            drop(responses);

            // 写入合并后的日志
            let entry = LlmLogEntry {
                timestamp,
                log_type: "response".to_string(),
                api: "chat_completions".to_string(),
                request_id: None,
                data: merged_data,
            };

            if let Err(e) = self.write_entry(&entry).await {
                error!("Failed to write merged LLM log: {}", e);
            } else {
                debug!(
                    "Merged LLM log written: chat_completions response (id: {})",
                    response_id
                );
            }
        }
    }

    /// 构建合并后的响应数据
    fn build_merged_response(&self, merging: &MergingResponse) -> Value {
        let mut result = merging.base_data.clone();

        // 修改choices数组，将累积的内容放入delta中
        if let Some(choices) = result.get_mut("choices").and_then(|v| v.as_array_mut()) {
            if let Some(choice) = choices.first_mut() {
                let mut delta = serde_json::Map::new();
                delta.insert("role".to_string(), Value::String("assistant".to_string()));

                if !merging.accumulated_content.is_empty() {
                    delta.insert(
                        "content".to_string(),
                        Value::String(merging.accumulated_content.clone()),
                    );
                }

                if !merging.accumulated_reasoning.is_empty() {
                    delta.insert(
                        "reasoning_content".to_string(),
                        Value::String(merging.accumulated_reasoning.clone()),
                    );
                }

                if let Some(choice_obj) = choice.as_object_mut() {
                    choice_obj.insert("delta".to_string(), Value::Object(delta));
                }
            }
        }

        result
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

/// 记录 Chat Completions API 响应（旧版本，直接写入）
///
/// 注意：此函数已被 `log_chat_response_merged` 替代，仅保留作为兼容性接口
#[allow(dead_code)]
pub async fn log_chat_response(request_id: Option<String>, data: Value) {
    LOGGER
        .log_response("chat_completions", request_id, data)
        .await;
}

/// 记录 Chat Completions API 响应（合并版本，推荐使用）
///
/// 此函数会将同一个response_id的多个SSE chunk合并成一条完整的日志记录
pub async fn log_chat_response_merged(data: Value) {
    LOGGER.log_chat_response_merged(data).await;
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

    #[tokio::test]
    async fn test_chat_response_merging() {
        let logger = LlmLogger::new();

        // 模拟流式响应的多个chunk
        let chunk1 = json!({
            "id": "test-response-123",
            "created": 1234567890,
            "model": "test-model",
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "content": "",
                    "reasoning_content": "首先"
                }
            }]
        });

        let chunk2 = json!({
            "id": "test-response-123",
            "created": 1234567890,
            "model": "test-model",
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "reasoning_content": "我需要"
                }
            }]
        });

        let chunk3 = json!({
            "id": "test-response-123",
            "created": 1234567890,
            "model": "test-model",
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "content": "这是"
                }
            }]
        });

        let chunk4 = json!({
            "id": "test-response-123",
            "created": 1234567890,
            "model": "test-model",
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "content": "测试"
                },
                "finish_reason": "stop"
            }]
        });

        // 记录所有chunk
        logger.log_chat_response_merged(chunk1).await;
        logger.log_chat_response_merged(chunk2).await;
        logger.log_chat_response_merged(chunk3).await;
        logger.log_chat_response_merged(chunk4).await;

        // 验证文件存在
        let log_path = logger.get_log_file_path();
        assert!(log_path.exists());

        // 读取日志文件，验证只有一条合并后的记录
        let content = tokio::fs::read_to_string(&log_path).await.unwrap();
        let lines: Vec<&str> = content.lines().collect();

        // 查找包含test-response-123的行
        let merged_line = lines
            .iter()
            .find(|line| line.contains("test-response-123"))
            .expect("应该找到合并后的日志");

        let log_entry: serde_json::Value = serde_json::from_str(merged_line).unwrap();

        // 验证合并后的内容
        let choices = log_entry["data"]["choices"].as_array().unwrap();
        let delta = &choices[0]["delta"];

        assert_eq!(delta["content"].as_str().unwrap(), "这是测试");
        assert_eq!(delta["reasoning_content"].as_str().unwrap(), "首先我需要");
    }
}
