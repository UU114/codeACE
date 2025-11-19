use crate::codex::Session;
use crate::codex::TurnContext;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::models::ResponseItem;
use tracing::warn;

/// Process streamed `ResponseItem`s from the model into the pair of:
/// - items we should record in conversation history; and
/// - `ResponseInputItem`s to send back to the model on the next turn.
pub(crate) async fn process_items(
    processed_items: Vec<crate::codex::ProcessedResponseItem>,
    sess: &Session,
    turn_context: &TurnContext,
) -> (Vec<ResponseInputItem>, Vec<ResponseItem>) {
    let mut items_to_record_in_conversation_history = Vec::<ResponseItem>::new();
    let mut responses = Vec::<ResponseInputItem>::new();
    let mut skipped_items_count = 0;

    for processed_response_item in processed_items {
        let crate::codex::ProcessedResponseItem { item, response } = processed_response_item;
        let matched = match (&item, &response) {
            (ResponseItem::Message { role, .. }, None) if role == "assistant" => {
                // If the model returned a message, we need to record it.
                items_to_record_in_conversation_history.push(item);
                true
            }
            (
                ResponseItem::LocalShellCall { .. },
                Some(ResponseInputItem::FunctionCallOutput { call_id, output }),
            ) => {
                items_to_record_in_conversation_history.push(item);
                items_to_record_in_conversation_history.push(ResponseItem::FunctionCallOutput {
                    call_id: call_id.clone(),
                    output: output.clone(),
                });
                true
            }
            (
                ResponseItem::FunctionCall { .. },
                Some(ResponseInputItem::FunctionCallOutput { call_id, output }),
            ) => {
                items_to_record_in_conversation_history.push(item);
                items_to_record_in_conversation_history.push(ResponseItem::FunctionCallOutput {
                    call_id: call_id.clone(),
                    output: output.clone(),
                });
                true
            }
            (
                ResponseItem::CustomToolCall { .. },
                Some(ResponseInputItem::CustomToolCallOutput { call_id, output }),
            ) => {
                items_to_record_in_conversation_history.push(item);
                items_to_record_in_conversation_history.push(ResponseItem::CustomToolCallOutput {
                    call_id: call_id.clone(),
                    output: output.clone(),
                });
                true
            }
            (
                ResponseItem::FunctionCall { .. },
                Some(ResponseInputItem::McpToolCallOutput { call_id, result }),
            ) => {
                items_to_record_in_conversation_history.push(item);
                let output = match result {
                    Ok(call_tool_result) => FunctionCallOutputPayload::from(call_tool_result),
                    Err(err) => FunctionCallOutputPayload {
                        content: err.clone(),
                        success: Some(false),
                        ..Default::default()
                    },
                };
                items_to_record_in_conversation_history.push(ResponseItem::FunctionCallOutput {
                    call_id: call_id.clone(),
                    output,
                });
                true
            }
            (
                ResponseItem::Reasoning {
                    id,
                    summary,
                    content,
                    encrypted_content,
                },
                None,
            ) => {
                items_to_record_in_conversation_history.push(ResponseItem::Reasoning {
                    id: id.clone(),
                    summary: summary.clone(),
                    content: content.clone(),
                    encrypted_content: encrypted_content.clone(),
                });
                true
            }
            _ => {
                // 不匹配的 ResponseItem，记录详细信息
                skipped_items_count += 1;

                // 提取类型信息用于诊断
                let item_type = match &item {
                    ResponseItem::Message { role, .. } => format!("Message(role={role})"),
                    ResponseItem::FunctionCall { name, .. } => {
                        format!("FunctionCall(name={name})")
                    }
                    ResponseItem::LocalShellCall { .. } => "LocalShellCall".to_string(),
                    ResponseItem::CustomToolCall { name, .. } => {
                        format!("CustomToolCall(tool={name})")
                    }
                    ResponseItem::FunctionCallOutput { .. } => "FunctionCallOutput".to_string(),
                    ResponseItem::CustomToolCallOutput { .. } => "CustomToolCallOutput".to_string(),
                    ResponseItem::Reasoning { .. } => "Reasoning".to_string(),
                    ResponseItem::WebSearchCall { .. } => "WebSearchCall".to_string(),
                    ResponseItem::GhostSnapshot { .. } => "GhostSnapshot".to_string(),
                    ResponseItem::Other => "Other".to_string(),
                };

                let response_type = match &response {
                    Some(ResponseInputItem::FunctionCallOutput { .. }) => {
                        "FunctionCallOutput".to_string()
                    }
                    Some(ResponseInputItem::CustomToolCallOutput { .. }) => {
                        "CustomToolCallOutput".to_string()
                    }
                    Some(ResponseInputItem::McpToolCallOutput { .. }) => {
                        "McpToolCallOutput".to_string()
                    }
                    Some(ResponseInputItem::Message { .. }) => "Message".to_string(),
                    None => "None".to_string(),
                };

                warn!(
                    "Unexpected response item pattern - item_type: {}, response_type: {}. This item will be skipped. Full item: {:?}, Full response: {:?}",
                    item_type, response_type, item, response
                );

                // 发送后台事件通知客户端
                let _ = sess
                    .send_event(
                        turn_context,
                        crate::protocol::EventMsg::BackgroundEvent(
                            crate::protocol::BackgroundEventEvent {
                                message: format!(
                                    "Warning: Skipped unexpected response item (type: {item_type}, response: {response_type})"
                                ),
                            },
                        ),
                    )
                    .await;

                false
            }
        };

        if matched && let Some(response) = response {
            responses.push(response);
        }
    }

    // 记录处理统计信息
    if skipped_items_count > 0 {
        tracing::warn!(
            "process_items summary: {} items skipped, {} items recorded, {} responses generated",
            skipped_items_count,
            items_to_record_in_conversation_history.len(),
            responses.len()
        );
    } else {
        tracing::debug!(
            "process_items summary: {} items recorded, {} responses generated",
            items_to_record_in_conversation_history.len(),
            responses.len()
        );
    }

    // Only attempt to take the lock if there is something to record.
    if !items_to_record_in_conversation_history.is_empty() {
        sess.record_conversation_items(turn_context, &items_to_record_in_conversation_history)
            .await;
    }
    (responses, items_to_record_in_conversation_history)
}
