//! Mission 模块 - 支持 Mission → TodoList → Tasks 工作流
//!
//! 这个模块实现了任务分解和跟踪机制：
//! - Mission: 用户提出的高层次任务
//! - TodoList: LLM 分解的步骤列表
//! - 每个 Todo 完成时触发 Reflector 生成 Bullet

pub mod manager;
pub mod types;

pub use manager::MissionManager;
pub use types::MissionContext;
pub use types::MissionStatus;
pub use types::TodoItem;
