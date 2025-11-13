//! Hook机制，用于扩展Executor功能
//!
//! 这个模块提供了一个最小化的扩展点，允许ACE等插件
//! 在不修改核心代码的情况下扩展功能。

use std::sync::Arc;

/// Executor扩展Hook trait
///
/// 实现这个trait可以在Executor执行前后注入自定义逻辑。
/// 所有的hook方法都是可选的，默认实现为空操作。
pub trait ExecutorHook: Send + Sync {
    /// 在执行查询前调用
    ///
    /// 返回的字符串将作为系统上下文添加到对话中。
    /// 如果返回None，则不添加任何上下文。
    fn pre_execute(&self, _query: &str) -> Option<String> {
        None
    }

    /// 在执行完成后调用
    ///
    /// 用于记录、学习或其他后处理逻辑。
    /// 注意：这个方法不应该阻塞主流程。
    fn post_execute(&self, _query: &str, _response: &str, _success: bool) {
        // 默认空实现
    }
}

/// Hook管理器
///
/// 管理和调用所有注册的hooks。
#[derive(Default)]
pub struct HookManager {
    hooks: Vec<Arc<dyn ExecutorHook>>,
}

impl HookManager {
    /// 创建新的Hook管理器
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    /// 注册一个新的hook
    pub fn register(&mut self, hook: Arc<dyn ExecutorHook>) {
        self.hooks.push(hook);
    }

    /// 调用所有pre_execute hooks
    ///
    /// 返回第一个非None的上下文，如果所有hooks都返回None则返回None。
    pub fn call_pre_execute(&self, query: &str) -> Option<String> {
        for hook in &self.hooks {
            if let Some(context) = hook.pre_execute(query) {
                tracing::debug!("Hook provided context: {} chars", context.len());
                return Some(context);
            }
        }
        None
    }

    /// 调用所有post_execute hooks
    ///
    /// 异步调用所有注册的hooks，不等待完成。
    pub fn call_post_execute(&self, query: &str, response: &str, success: bool) {
        for hook in &self.hooks {
            let hook_clone = Arc::clone(hook);
            let query_clone = query.to_string();
            let response_clone = response.to_string();

            // 在新的任务中异步调用，避免阻塞
            tokio::spawn(async move {
                hook_clone.post_execute(&query_clone, &response_clone, success);
            });
        }
    }

    /// 获取已注册的hook数量
    pub fn hook_count(&self) -> usize {
        self.hooks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHook {
        context: String,
    }

    impl ExecutorHook for TestHook {
        fn pre_execute(&self, _query: &str) -> Option<String> {
            Some(self.context.clone())
        }

        fn post_execute(&self, _query: &str, _response: &str, _success: bool) {
            // 测试实现
        }
    }

    #[test]
    fn test_hook_manager() {
        let mut manager = HookManager::new();
        assert_eq!(manager.hook_count(), 0);

        let hook = Arc::new(TestHook {
            context: "test context".to_string(),
        });

        manager.register(hook);
        assert_eq!(manager.hook_count(), 1);

        let query = "test query";

        let context = manager.call_pre_execute(query);
        assert_eq!(context, Some("test context".to_string()));
    }
}
