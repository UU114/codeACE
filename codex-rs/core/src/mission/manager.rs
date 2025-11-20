//! Mission Manager - Mission Lifecycle Management
//!
//! Responsible for creating, updating and tracking Mission status

use super::types::MissionContext;
use super::types::TodoItem;
use codex_protocol::plan_tool::StepStatus;

/// Mission Manager
///
/// Manages Mission creation, updates and status tracking
pub struct MissionManager {
    /// Currently active Mission (if any)
    current_mission: Option<MissionContext>,
}

impl MissionManager {
    /// Create new Mission Manager
    pub fn new() -> Self {
        Self {
            current_mission: None,
        }
    }

    /// Start new Mission
    ///
    /// If there's an active Mission, complete it first
    pub fn start_mission(&mut self, description: String, session_id: String) -> &MissionContext {
        // If there's existing Mission, mark it as complete
        if let Some(ref mut mission) = self.current_mission {
            tracing::info!("Completing previous mission: {}", mission.description);
        }

        // Create new Mission
        let mission = MissionContext::new(description.clone(), session_id);
        tracing::info!("Started new mission: {}", description);

        self.current_mission = Some(mission);
        self.current_mission.as_ref().unwrap()
    }

    /// Update TodoList
    ///
    /// Auto-create Mission if no active Mission exists
    /// Return newly completed Todo items (need to trigger Reflector)
    pub fn update_todos(
        &mut self,
        steps: Vec<(String, StepStatus)>,
        session_id: String,
    ) -> Vec<TodoItem> {
        // Create Mission if no current Mission exists
        if self.current_mission.is_none() {
            self.start_mission("Untitled Mission".to_string(), session_id);
        }

        // Update todos
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

    /// Mark Todo as reflected
    pub fn mark_todo_reflected(&mut self, todo_id: &str) {
        if let Some(ref mut mission) = self.current_mission {
            mission.mark_todo_reflected(todo_id);
            tracing::debug!("Marked todo {} as reflected", todo_id);
        }
    }

    /// Get current Mission
    pub fn current_mission(&self) -> Option<&MissionContext> {
        self.current_mission.as_ref()
    }

    /// Get current Mission (mutable)
    pub fn current_mission_mut(&mut self) -> Option<&mut MissionContext> {
        self.current_mission.as_mut()
    }

    /// Complete current Mission
    pub fn complete_current_mission(&mut self) {
        if let Some(ref mission) = self.current_mission {
            tracing::info!("Completed mission: {}", mission.description);
        }
        self.current_mission = None;
    }

    /// Get unreflected completed Todos
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

        manager.start_mission("Write Sudoku Game".to_string(), "session-123".to_string());

        let mission = manager.current_mission().unwrap();
        assert_eq!(mission.description, "Write Sudoku Game");
    }

    #[test]
    fn test_update_todos_creates_mission() {
        let mut manager = MissionManager::new();

        let steps = vec![
            ("Step1".to_string(), StepStatus::Completed),
            ("Step2".to_string(), StepStatus::Pending),
        ];

        let newly_completed = manager.update_todos(steps, "session-123".to_string());

        // Mission should be auto-created
        assert!(manager.current_mission().is_some());
        assert_eq!(newly_completed.len(), 1);
    }

    #[test]
    fn test_update_todos_returns_newly_completed() {
        let mut manager = MissionManager::new();
        manager.start_mission("Test Task".to_string(), "session-123".to_string());

        // First update
        let steps = vec![
            ("Step1".to_string(), StepStatus::Completed),
            ("Step2".to_string(), StepStatus::Pending),
        ];
        let newly_completed = manager.update_todos(steps, "session-123".to_string());
        assert_eq!(newly_completed.len(), 1);
        assert_eq!(newly_completed[0].step, "Step1");

        // Second update: Step2 completed
        let steps = vec![
            ("Step1".to_string(), StepStatus::Completed),
            ("Step2".to_string(), StepStatus::Completed),
        ];
        let newly_completed = manager.update_todos(steps, "session-123".to_string());
        assert_eq!(newly_completed.len(), 1);
        assert_eq!(newly_completed[0].step, "Step2");
    }

    #[test]
    fn test_mark_todo_reflected() {
        let mut manager = MissionManager::new();

        let steps = vec![("Step1".to_string(), StepStatus::Completed)];
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
            ("Step1".to_string(), StepStatus::Completed),
            ("Step2".to_string(), StepStatus::Completed),
            ("Step3".to_string(), StepStatus::Pending),
        ];
        manager.update_todos(steps, "session-123".to_string());

        // Mark first one as reflected
        let mission = manager.current_mission().unwrap();
        let todo_id = mission.todos[0].id.clone();
        manager.mark_todo_reflected(&todo_id);

        // Should only return the second one
        let unreflected = manager.get_unreflected_completed_todos();
        assert_eq!(unreflected.len(), 1);
        assert_eq!(unreflected[0].step, "Step2");
    }

    #[test]
    fn test_complete_current_mission() {
        let mut manager = MissionManager::new();
        manager.start_mission("Test Task".to_string(), "session-123".to_string());

        assert!(manager.current_mission().is_some());

        manager.complete_current_mission();
        assert!(manager.current_mission().is_none());
    }

    #[test]
    fn test_start_mission_replaces_previous() {
        let mut manager = MissionManager::new();

        manager.start_mission("Task1".to_string(), "session-1".to_string());
        assert_eq!(manager.current_mission().unwrap().description, "Task1");

        manager.start_mission("Task2".to_string(), "session-2".to_string());
        assert_eq!(manager.current_mission().unwrap().description, "Task2");
    }
}
