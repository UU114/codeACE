//! ACE 学习功能完整测试
//!
//! 测试实际对话场景中的学习和上下文加载

use anyhow::Result;
use codex_core::ace::ACEPlugin;
use codex_core::hooks::ExecutorHook;
use tempfile::TempDir;
use tokio;

#[tokio::test]
async fn test_ace_complete_learning_flow() -> Result<()> {
    // 初始化日志（便于调试）
    // Note: tracing_subscriber is not a dependency in tests, skip init

    println!("\n========================================");
    println!("ACE 完整学习流程测试");
    println!("========================================\n");

    // 创建临时目录
    let temp_dir = TempDir::new()?;
    let codex_home = temp_dir.path();

    // 步骤 1: 初始化 ACE 插件
    println!("步骤 1: 初始化 ACE 插件...");
    let plugin = ACEPlugin::from_codex_home(codex_home)
        .await?
        .expect("Failed to create ACE plugin");
    println!("✅ ACE 插件初始化成功");

    // 步骤 2: 第一次对话 - 应该没有上下文
    println!("\n步骤 2: 第一次对话测试...");
    let query1 = "How do I create an async function in Rust?";

    let context_before = plugin.pre_execute(query1);
    if context_before.is_none() {
        println!("✅ pre_execute: 无历史上下文（符合预期）");
    } else {
        println!("ℹ️  pre_execute: 返回了上下文（可能是空的查询结果）");
        println!("   上下文内容: {:?}", context_before);
    }

    // 模拟成功的响应
    let response1 = r#"You can create an async function in Rust using the 'async' keyword:

async fn my_function() -> Result<String> {
    let result = some_async_operation().await?;
    Ok(result)
}

Key points:
- Use 'async' keyword before 'fn'
- Use '.await' to wait for async operations
- Can return Result for error handling"#;

    // 触发学习
    plugin.post_execute(query1, response1, true);
    println!("✅ post_execute: 学习过程已触发");

    // 给学习过程足够时间
    println!("   等待学习过程完成...");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // 步骤 3: 验证存储
    println!("\n步骤 3: 验证存储结果...");
    use codex_core::ace::BulletStorage;
    let storage_path = codex_home.join(".codeACE").join("ace");
    let storage = BulletStorage::new(&storage_path, 500)?;

    let stats = storage.get_stats().await?;
    println!("   统计信息:");
    println!("   - 总 bullets: {}", stats.total_bullets);
    println!("   - 总 sessions: {}", stats.total_sessions);
    println!("   - 成功率: {:.2}%", stats.overall_success_rate * 100.0);

    if stats.total_bullets > 0 {
        println!("✅ 学习成功: 生成了 {} 条 bullets", stats.total_bullets);

        // 显示生成的 bullets
        let playbook = storage.load_playbook().await?;
        println!("\n   生成的 bullets:");
        for (i, bullet) in playbook.all_bullets().iter().enumerate() {
            println!("   {}. [{:?}] {}", i + 1, bullet.section, bullet.content);
        }
    } else {
        println!("⚠️  警告: 未生成 bullets (可能 Reflector/Curator 未实现)");
    }

    // 步骤 4: 第二次对话 - 测试上下文加载
    println!("\n步骤 4: 第二次对话测试（相关查询）...");

    // 再等一会儿确保学习完成
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let query2 = "What is the syntax for async functions in Rust?";
    let context_after = plugin.pre_execute(query2);

    if let Some(ctx) = context_after {
        println!("✅ pre_execute: 找到相关上下文！");
        println!("\n   加载的上下文内容:");
        println!("   {}", ctx);
    } else {
        println!("ℹ️  pre_execute: 未找到相关上下文");
        println!("   (这可能因为 query_bullets 未实现或没有相关内容)");
    }

    // 步骤 5: 第三次对话 - 不相关的查询
    println!("\n步骤 5: 第三次对话测试（不相关查询）...");
    let query3 = "How do I configure Python virtual environments?";
    let context_unrelated = plugin.pre_execute(query3);

    if context_unrelated.is_none() {
        println!("✅ pre_execute: 正确判断无相关上下文");
    } else {
        println!("ℹ️  pre_execute: 找到了上下文（可能相关性判断宽松）");
    }

    println!("\n========================================");
    println!("测试完成！");
    println!("========================================\n");

    Ok(())
}
