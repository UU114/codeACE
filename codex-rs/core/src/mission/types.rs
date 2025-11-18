//! Mission 相关的数据类型定义
//!
//! 支持 Mission → TodoList → Tasks 工作流

use chrono::DateTime;
use chrono::Utc;
use codex_protocol::plan_tool::StepStatus;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

/// Mission 上下文
///
/// 跟踪用户提出的高层次任务（Mission）和对应的 TodoList
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionContext {
    /// Mission 唯一标识符
    pub id: String,

    /// Mission 描述（用户的原始需求）
    pub description: String,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 最后更新时间
    pub updated_at: DateTime<Utc>,

    /// TodoList 中的所有 Todo 项
    pub todos: Vec<TodoItem>,

    /// Mission 状态
    pub status: MissionStatus,

    /// 会话ID（首次创建时）
    pub source_session_id: String,
}

/// Mission 状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MissionStatus {
    /// 进行中
    Active,

    /// 已完成
    Completed,

    /// 已取消
    Cancelled,
}

/// Todo 项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    /// Todo 唯一标识符
    pub id: String,

    /// Todo 描述
    pub step: String,

    /// 当前状态
    pub status: StepStatus,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 最后更新时间
    pub updated_at: DateTime<Utc>,

    /// 是否已触发 Reflector
    ///
    /// 用于去重，确保每个 Todo 完成时只触发一次 Reflector
    pub reflected: bool,

    /// 完成时间（如果已完成）
    pub completed_at: Option<DateTime<Utc>>,
}

impl MissionContext {
    /// 创建新的 Mission 上下文
    pub fn new(description: String, session_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            description,
            created_at: now,
            updated_at: now,
            todos: Vec::new(),
            status: MissionStatus::Active,
            source_session_id: session_id,
        }
    }

    /// 更新 TodoList
    ///
    /// 返回新完成的 Todo 项列表（用于触发 Reflector）
    pub fn update_todos(&mut self, new_steps: Vec<(String, StepStatus)>) -> Vec<TodoItem> {
        let now = Utc::now();
        self.updated_at = now;

        // 记录之前已完成的 todo 的 step 描述
        let previously_completed: std::collections::HashSet<String> = self
            .todos
            .iter()
            .filter(|t| matches!(t.status, StepStatus::Completed))
            .map(|t| t.step.clone())
            .collect();

        // 清空现有的 todos（会被新的列表替换）
        self.todos.clear();

        // 创建新的 todos
        let mut newly_completed = Vec::new();

        for (step, status) in new_steps {
            let is_newly_completed =
                matches!(status, StepStatus::Completed) && !previously_completed.contains(&step);

            let is_completed = matches!(status, StepStatus::Completed);

            let todo = TodoItem {
                id: Uuid::new_v4().to_string(),
                step: step.clone(),
                status,
                created_at: now,
                updated_at: now,
                reflected: false,
                completed_at: if is_completed { Some(now) } else { None },
            };

            if is_newly_completed {
                newly_completed.push(todo.clone());
            }

            self.todos.push(todo);
        }

        // 更新 Mission 状态
        self.update_mission_status();

        newly_completed
    }

    /// 标记 Todo 为已反射
    pub fn mark_todo_reflected(&mut self, todo_id: &str) {
        if let Some(todo) = self.todos.iter_mut().find(|t| t.id == todo_id) {
            todo.reflected = true;
            todo.updated_at = Utc::now();
        }
        self.updated_at = Utc::now();
    }

    /// 更新 Mission 状态（基于 Todos 的状态）
    fn update_mission_status(&mut self) {
        if self.todos.is_empty() {
            return;
        }

        // 如果所有 todos 都完成了，Mission 也标记为完成
        let all_completed = self
            .todos
            .iter()
            .all(|t| matches!(t.status, StepStatus::Completed));

        if all_completed {
            self.status = MissionStatus::Completed;
        } else {
            self.status = MissionStatus::Active;
        }
    }

    /// 获取未反射的已完成 Todos
    pub fn get_unreflected_completed_todos(&self) -> Vec<&TodoItem> {
        self.todos
            .iter()
            .filter(|t| matches!(t.status, StepStatus::Completed) && !t.reflected)
            .collect()
    }

    /// 是否已完成
    pub fn is_completed(&self) -> bool {
        self.status == MissionStatus::Completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mission_creation() {
        let mission = MissionContext::new("编写数独游戏".to_string(), "session-123".to_string());

        assert_eq!(mission.description, "编写数独游戏");
        assert_eq!(mission.status, MissionStatus::Active);
        assert!(mission.todos.is_empty());
    }

    #[test]
    fn test_update_todos() {
        let mut mission = MissionContext::new("测试任务".to_string(), "session-123".to_string());

        let steps = vec![
            ("选择技术栈".to_string(), StepStatus::Completed),
            ("设计架构".to_string(), StepStatus::InProgress),
            ("开发代码".to_string(), StepStatus::Pending),
        ];

        let newly_completed = mission.update_todos(steps);

        assert_eq!(mission.todos.len(), 3);
        assert_eq!(newly_completed.len(), 1);
        assert_eq!(newly_completed[0].step, "选择技术栈");
    }

    #[test]
    fn test_mission_status_update() {
        let mut mission = MissionContext::new("测试任务".to_string(), "session-123".to_string());

        // 开始时，Mission 是 Active
        let steps = vec![
            ("步骤1".to_string(), StepStatus::Completed),
            ("步骤2".to_string(), StepStatus::Pending),
        ];
        mission.update_todos(steps);
        assert_eq!(mission.status, MissionStatus::Active);

        // 全部完成后，Mission 也完成
        let steps = vec![
            ("步骤1".to_string(), StepStatus::Completed),
            ("步骤2".to_string(), StepStatus::Completed),
        ];
        mission.update_todos(steps);
        assert_eq!(mission.status, MissionStatus::Completed);
        assert!(mission.is_completed());
    }

    #[test]
    fn test_mark_todo_reflected() {
        let mut mission = MissionContext::new("测试任务".to_string(), "session-123".to_string());

        let steps = vec![("步骤1".to_string(), StepStatus::Completed)];
        mission.update_todos(steps);

        let todo_id = mission.todos[0].id.clone();
        assert!(!mission.todos[0].reflected);

        mission.mark_todo_reflected(&todo_id);
        assert!(mission.todos[0].reflected);
    }

    #[test]
    fn test_get_unreflected_completed_todos() {
        let mut mission = MissionContext::new("测试任务".to_string(), "session-123".to_string());

        let steps = vec![
            ("步骤1".to_string(), StepStatus::Completed),
            ("步骤2".to_string(), StepStatus::Completed),
            ("步骤3".to_string(), StepStatus::Pending),
        ];
        mission.update_todos(steps);

        // 标记第一个为已反射
        let todo_id = mission.todos[0].id.clone();
        mission.mark_todo_reflected(&todo_id);

        // 应该只返回第二个已完成但未反射的
        let unreflected = mission.get_unreflected_completed_todos();
        assert_eq!(unreflected.len(), 1);
        assert_eq!(unreflected[0].step, "步骤2");
    }

    #[test]
    fn test_newly_completed_detection() {
        let mut mission = MissionContext::new("测试任务".to_string(), "session-123".to_string());

        // 第一次更新：步骤1完成
        let steps = vec![
            ("步骤1".to_string(), StepStatus::Completed),
            ("步骤2".to_string(), StepStatus::Pending),
        ];
        let newly_completed = mission.update_todos(steps);
        assert_eq!(newly_completed.len(), 1);
        assert_eq!(newly_completed[0].step, "步骤1");

        // 第二次更新：步骤2完成，步骤1仍然完成（不应重复计数）
        let steps = vec![
            ("步骤1".to_string(), StepStatus::Completed),
            ("步骤2".to_string(), StepStatus::Completed),
        ];
        let newly_completed = mission.update_todos(steps);
        assert_eq!(newly_completed.len(), 1);
        assert_eq!(newly_completed[0].step, "步骤2");
    }
}
