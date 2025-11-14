use std::sync::Arc;

use crate::AuthManager;
use crate::RolloutRecorder;
use crate::mcp_connection_manager::McpConnectionManager;
use crate::tools::sandboxing::ApprovalStore;
use crate::unified_exec::UnifiedExecSessionManager;
use crate::user_notification::UserNotifier;
use codex_otel::otel_event_manager::OtelEventManager;
use tokio::sync::Mutex;

pub(crate) struct SessionServices {
    pub(crate) mcp_connection_manager: McpConnectionManager,
    pub(crate) unified_exec_manager: UnifiedExecSessionManager,
    pub(crate) notifier: UserNotifier,
    pub(crate) rollout: Mutex<Option<RolloutRecorder>>,
    pub(crate) user_shell: crate::shell::Shell,
    pub(crate) show_raw_agent_reasoning: bool,
    pub(crate) auth_manager: Arc<AuthManager>,
    pub(crate) otel_event_manager: OtelEventManager,
    pub(crate) tool_approvals: Mutex<ApprovalStore>,
    #[cfg(feature = "ace")]
    #[allow(dead_code)] // Hook功能暂未完全集成
    pub(crate) hook_manager: Option<Arc<crate::hooks::HookManager>>,
    /// ACE Plugin 直接引用（用于 Mission/Todo 触发等高级功能）
    #[cfg(feature = "ace")]
    pub(crate) ace_plugin: Option<Arc<crate::ace::ACEPlugin>>,
    /// Mission Manager（Mission → TodoList → Tasks 工作流）
    #[cfg(feature = "ace")]
    pub(crate) mission_manager: Mutex<crate::mission::MissionManager>,
}
