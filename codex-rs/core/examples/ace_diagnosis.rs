// ACE Playbook上下文注入诊断工具
//
// 这个工具用于诊断为什么playbook内容没有被注入到LLM对话中
//
// 使用方法：
// 1. 将此文件放到 codex-rs/core/examples/ 目录
// 2. 运行: cargo run --example ace_diagnosis --features ace

use codex_core::ace::{ACEConfig, ACEPlugin, BulletStorage};
use codex_core::hooks::ExecutorHook;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("===== ACE Playbook诊断工具 =====\n");

    // 1. 检查playbook文件是否存在
    let ace_path = shellexpand::tilde("~/.codeACE/ace").to_string();
    let ace_path = PathBuf::from(ace_path);
    let playbook_path = ace_path.join("playbook.json");

    println!("1. 检查Playbook文件:");
    println!("   路径: {:?}", playbook_path);
    println!("   存在: {}", playbook_path.exists());

    if !playbook_path.exists() {
        println!("   ❌ Playbook文件不存在！");
        return Ok(());
    }
    println!("   ✅ Playbook文件存在\n");

    // 2. 加载playbook并统计bullets数量
    println!("2. 加载Playbook数据:");
    let storage = BulletStorage::new(&ace_path, 500)?;
    let playbook = storage.load_playbook().await?;

    let all_bullets = playbook.all_bullets();
    println!("   总bullet数: {}", all_bullets.len());

    for (section, bullets) in playbook.bullets.iter() {
        println!("   - {:?}: {} 条", section, bullets.len());
    }
    println!();

    // 3. 测试查询功能
    println!("3. 测试查询功能:");

    let test_queries = vec![
        "rust test",              // 应该匹配playbook中的内容
        "pm2 部署",               // 应该匹配
        "analyse game",           // 应该匹配
        "随机不存在的内容xyz123", // 不应该匹配
    ];

    for query in test_queries {
        println!("\n   查询: \"{}\"", query);
        match storage.query_bullets(query, 5).await {
            Ok(bullets) if !bullets.is_empty() => {
                println!("   ✅ 找到 {} 条相关bullets:", bullets.len());
                for (i, bullet) in bullets.iter().enumerate().take(2) {
                    println!(
                        "      {}. {}",
                        i + 1,
                        bullet.content.chars().take(60).collect::<String>()
                    );
                }
            }
            Ok(_) => {
                println!("   ⚠️  没有找到相关bullets");
            }
            Err(e) => {
                println!("   ❌ 查询失败: {}", e);
            }
        }
    }
    println!();

    // 4. 测试ACE Plugin的pre_execute hook
    println!("4. 测试ACE Plugin Hook:");

    let config = ACEConfig {
        enabled: true,
        storage_path: ace_path.to_str().unwrap().to_string(),
        max_entries: 500,
        ..Default::default()
    };

    let plugin = ACEPlugin::new(config)?;
    println!("   ✅ ACE Plugin创建成功");

    // 测试pre_execute
    let test_query = "如何运行rust测试";
    println!("\n   测试查询: \"{}\"", test_query);

    match plugin.pre_execute(test_query) {
        Some(context) => {
            println!("   ✅ pre_execute返回了上下文!");
            println!("   上下文长度: {} 字符", context.len());
            println!("   上下文预览:");
            for line in context.lines().take(10) {
                println!("      {}", line);
            }
        }
        None => {
            println!("   ❌ pre_execute返回None（没有找到相关上下文）");
        }
    }

    println!("\n===== 诊断完成 =====");

    Ok(())
}
