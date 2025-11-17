use crate::client_common::tools::ResponsesApiTool;
use crate::client_common::tools::ToolSpec;
use crate::codex::Session;
use crate::codex::TurnContext;
use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use crate::tools::spec::JsonSchema;
use async_trait::async_trait;
use codex_protocol::plan_tool::UpdatePlanArgs;
use codex_protocol::protocol::EventMsg;
use std::collections::BTreeMap;
use std::sync::LazyLock;

pub struct PlanHandler;

pub static PLAN_TOOL: LazyLock<ToolSpec> = LazyLock::new(|| {
    let mut plan_item_props = BTreeMap::new();
    plan_item_props.insert("step".to_string(), JsonSchema::String { description: None });
    plan_item_props.insert(
        "status".to_string(),
        JsonSchema::String {
            description: Some("One of: pending, in_progress, completed".to_string()),
        },
    );

    let plan_items_schema = JsonSchema::Array {
        description: Some("The list of steps".to_string()),
        items: Box::new(JsonSchema::Object {
            properties: plan_item_props,
            required: Some(vec!["step".to_string(), "status".to_string()]),
            additional_properties: Some(false.into()),
        }),
    };

    let mut properties = BTreeMap::new();
    properties.insert(
        "explanation".to_string(),
        JsonSchema::String { description: None },
    );
    properties.insert("plan".to_string(), plan_items_schema);

    ToolSpec::Function(ResponsesApiTool {
        name: "update_plan".to_string(),
        description: r#"Updates the task plan.
Provide an optional explanation and a list of plan items, each with a step and status.
At most one step can be in_progress at a time.
"#
        .to_string(),
        strict: false,
        parameters: JsonSchema::Object {
            properties,
            required: Some(vec!["plan".to_string()]),
            additional_properties: Some(false.into()),
        },
    })
});

#[async_trait]
impl ToolHandler for PlanHandler {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            call_id,
            payload,
            ..
        } = invocation;

        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "update_plan handler received unsupported payload".to_string(),
                ));
            }
        };

        let content =
            handle_update_plan(session.as_ref(), turn.as_ref(), arguments, call_id).await?;

        Ok(ToolOutput::Function {
            content,
            content_items: None,
            success: Some(true),
        })
    }
}

/// This function doesn't do anything useful. However, it gives the model a structured way to record its plan that clients can read and render.
/// So it's the _inputs_ to this function that are useful to clients, not the outputs and neither are actually useful for the model other
/// than forcing it to come up and document a plan (TBD how that affects performance).
pub(crate) async fn handle_update_plan(
    session: &Session,
    turn_context: &TurnContext,
    arguments: String,
    _call_id: String,
) -> Result<String, FunctionCallError> {
    let args = parse_update_plan_arguments(&arguments)?;

    // 发送 PlanUpdate 事件
    session
        .send_event(turn_context, EventMsg::PlanUpdate(args.clone()))
        .await;

    // Mission/Todo 处理（仅在 ACE 功能启用时）
    #[cfg(feature = "ace")]
    {
        handle_mission_todos(session, turn_context, &args).await;
    }

    Ok("Plan updated".to_string())
}

/// 处理 Mission/Todo 更新（ACE 功能）
#[cfg(feature = "ace")]
async fn handle_mission_todos(
    session: &Session,
    turn_context: &TurnContext,
    args: &UpdatePlanArgs,
) {
    // 1. 更新 MissionManager
    let newly_completed = {
        let mut mission_mgr = session.services.mission_manager.lock().await;
        let steps: Vec<(String, codex_protocol::plan_tool::StepStatus)> = args
            .plan
            .iter()
            .map(|item| (item.step.clone(), item.status.clone()))
            .collect();

        mission_mgr.update_todos(steps, turn_context.sub_id.clone())
    };

    // 2. 如果有新完成的 Todos，触发 Reflector
    if !newly_completed.is_empty() {
        if let Some(ref ace_plugin) = session.services.ace_plugin {
            for todo in newly_completed {
                tracing::info!("✅ Todo completed: {}", todo.step);

                // 构建对话上下文（包含 explanation 和 plan 信息）
                let conversation_context = build_todo_context(args, &todo);

                // 触发 Reflector
                ace_plugin.on_todo_completed(
                    todo.step.clone(),
                    conversation_context,
                    turn_context.sub_id.clone(),
                );

                // 标记为已反射
                let mut mission_mgr = session.services.mission_manager.lock().await;
                mission_mgr.mark_todo_reflected(&todo.id);
            }
        }
    }
}

/// 构建 Todo 完成的对话上下文
#[cfg(feature = "ace")]
fn build_todo_context(args: &UpdatePlanArgs, todo: &crate::mission::TodoItem) -> String {
    let mut context = String::new();

    // 添加 explanation（如果有）
    if let Some(ref explanation) = args.explanation {
        context.push_str("## Context\n");
        context.push_str(explanation);
        context.push_str("\n\n");
    }

    // 添加完整的 plan
    context.push_str("## Plan Overview\n");
    for (idx, item) in args.plan.iter().enumerate() {
        let status_symbol = match item.status {
            codex_protocol::plan_tool::StepStatus::Completed => "✅",
            codex_protocol::plan_tool::StepStatus::InProgress => "🔄",
            codex_protocol::plan_tool::StepStatus::Pending => "⏳",
        };
        let marker = if item.step == todo.step { "**" } else { "" };
        context.push_str(&format!(
            "{}{} {}. {}{}\n",
            marker,
            status_symbol,
            idx + 1,
            item.step,
            marker
        ));
    }

    context
}

fn parse_update_plan_arguments(arguments: &str) -> Result<UpdatePlanArgs, FunctionCallError> {
    serde_json::from_str::<UpdatePlanArgs>(arguments).map_err(|e| {
        FunctionCallError::RespondToModel(format!("failed to parse function arguments: {e}"))
    })
}
