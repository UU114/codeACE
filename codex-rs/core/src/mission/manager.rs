//! Mission Manager - Mission 生命周期管理
//!
//! 负责创建、更新和跟踪 Mission 状态

use super::types::{MissionContext, TodoItem};
use codex_protocol::plan_tool::StepStatus;

/// Mission Manager
///
/// 管理 Mission 的创建、更新和状态跟踪
pub struct MissionManager {
    /// 当前活跃的 Mission (如果有)
    current_mission: Option<MissionContext>,
}

impl MissionManager {
    /// 创建新的 Mission Manager
    pub fn new() -> Self {
        Self {
            current_mission: None,
        }
    }

    /// 启动新的 Mission
    ///
    /// 如果已有活跃的 Mission，会先完成它
    pub fn start_mission(&mut self, description: String, session_id: String) -> &MissionContext {
        // 如果有现有的 Mission，标记为完成
        if let Some(ref mut mission) = self.current_mission {
            tracing::info!("Completing previous mission: {}", mission.description);
        }

        // 创建新 Mission
        let mission = MissionContext::new(description.clone(), session_id);
        tracing::info!("Started new mission: {}", description);

        self.current_mission = Some(mission);
        self.current_mission.as_ref().unwrap()
    }

    /// 更新 TodoList
    ///
    /// 如果没有活跃的 Mission，会自动创建一个
    /// 返回新完成的 Todo 项（需要触发 Reflector 的）
    pub fn update_todos(
        &mut self,
        steps: Vec<(String, StepStatus)>,
        session_id: String,
    ) -> Vec<TodoItem> {
        // 如果没有当前 Mission，创建一个
        if self.current_mission.is_none() {
            self.start_mission("Untitled Mission".to_string(), session_id.clone());
        }

        // 更新 todos
        if let Some(ref mut mission) = self.current_mission {
            let newly_completed = mission.update_todos(steps);

            tracing::debug!(
                "Updated mission todos: {} total, {} newly completed",
                mission.todos.len(),
                newly_completed.len()
            );

            newly_completed
        } else {
            Vec::new()
        }
    }

    /// 标记 Todo 为已反射
    pub fn mark_todo_reflected(&mut self, todo_id: &str) {
        if let Some(ref mut mission) = self.current_mission {
            mission.mark_todo_reflected(todo_id);
            tracing::debug!("Marked todo {} as reflected", todo_id);
        }
    }

    /// 获取当前 Mission
    pub fn current_mission(&self) -> Option<&MissionContext> {
        self.current_mission.as_ref()
    }

    /// 获取当前 Mission (可变)
    pub fn current_mission_mut(&mut self) -> Option<&mut MissionContext> {
        self.current_mission.as_mut()
    }

    /// 完成当前 Mission
    pub fn complete_current_mission(&mut self) {
        if let Some(ref mission) = self.current_mission {
            tracing::info!("Completed mission: {}", mission.description);
        }
        self.current_mission = None;
    }

    /// 获取未反射的已完成 Todos
    pub fn get_unreflected_completed_todos(&self) -> Vec<&TodoItem> {
        self.current_mission
            .as_ref()
            .map(|m| m.get_unreflected_completed_todos())
            .unwrap_or_default()
    }
}

impl Default for MissionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_mission() {
        let mut manager = MissionManager::new();
        assert!(manager.current_mission().is_none());

        manager.start_mission("编写数独游戏".to_string(), "session-123".to_string());

        let mission = manager.current_mission().unwrap();
        assert_eq!(mission.description, "编写数独游戏");
    }

    #[test]
    fn test_update_todos_creates_mission() {
        let mut manager = MissionManager::new();

        let steps = vec![
            ("步骤1".to_string(), StepStatus::Completed),
            ("步骤2".to_string(), StepStatus::Pending),
        ];

        let newly_completed = manager.update_todos(steps, "session-123".to_string());

        // 应该自动创建了 Mission
        assert!(manager.current_mission().is_some());
        assert_eq!(newly_completed.len(), 1);
    }

    #[test]
    fn test_update_todos_returns_newly_completed() {
        let mut manager = MissionManager::new();
        manager.start_mission("测试任务".to_string(), "session-123".to_string());

        // 第一次更新
        let steps = vec![
            ("步骤1".to_string(), StepStatus::Completed),
            ("步骤2".to_string(), StepStatus::Pending),
        ];
        let newly_completed = manager.update_todos(steps, "session-123".to_string());
        assert_eq!(newly_completed.len(), 1);
        assert_eq!(newly_completed[0].step, "步骤1");

        // 第二次更新：步骤2完成
        let steps = vec![
            ("步骤1".to_string(), StepStatus::Completed),
            ("步骤2".to_string(), StepStatus::Completed),
        ];
        let newly_completed = manager.update_todos(steps, "session-123".to_string());
        assert_eq!(newly_completed.len(), 1);
        assert_eq!(newly_completed[0].step, "步骤2");
    }

    #[test]
    fn test_mark_todo_reflected() {
        let mut manager = MissionManager::new();

        let steps = vec![("步骤1".to_string(), StepStatus::Completed)];
        manager.update_todos(steps, "session-123".to_string());

        let mission = manager.current_mission().unwrap();
        let todo_id = mission.todos[0].id.clone();

        manager.mark_todo_reflected(&todo_id);

        let mission = manager.current_mission().unwrap();
        assert!(mission.todos[0].reflected);
    }

    #[test]
    fn test_get_unreflected_completed_todos() {
        let mut manager = MissionManager::new();

        let steps = vec![
            ("步骤1".to_string(), StepStatus::Completed),
            ("步骤2".to_string(), StepStatus::Completed),
            ("步骤3".to_string(), StepStatus::Pending),
        ];
        manager.update_todos(steps, "session-123".to_string());

        // 标记第一个为已反射
        let mission = manager.current_mission().unwrap();
        let todo_id = mission.todos[0].id.clone();
        manager.mark_todo_reflected(&todo_id);

        // 应该只返回第二个
        let unreflected = manager.get_unreflected_completed_todos();
        assert_eq!(unreflected.len(), 1);
        assert_eq!(unreflected[0].step, "步骤2");
    }

    #[test]
    fn test_complete_current_mission() {
        let mut manager = MissionManager::new();
        manager.start_mission("测试任务".to_string(), "session-123".to_string());

        assert!(manager.current_mission().is_some());

        manager.complete_current_mission();
        assert!(manager.current_mission().is_none());
    }

    #[test]
    fn test_start_mission_replaces_previous() {
        let mut manager = MissionManager::new();

        manager.start_mission("任务1".to_string(), "session-1".to_string());
        assert_eq!(manager.current_mission().unwrap().description, "任务1");

        manager.start_mission("任务2".to_string(), "session-2".to_string());
        assert_eq!(manager.current_mission().unwrap().description, "任务2");
    }
}
