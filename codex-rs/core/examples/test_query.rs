// 测试查询匹配的详细调试程序
use codex_core::ace::BulletStorage;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("===== 测试查询匹配详情 =====\n");

    let ace_path = shellexpand::tilde("~/.codeACE/ace").to_string();
    let ace_path = PathBuf::from(ace_path);
    let storage = BulletStorage::new(&ace_path, 500)?;

    let test_queries = vec![
        "rust test",        // 应该匹配
        "如何运行rust测试", // 目前不匹配
        "rust",             // 简单测试
        "测试",             // 中文单词
        "运行",             // 中文单词
        "cargo test",       // 工具匹配
    ];

    for query in test_queries {
        println!("\n查询: \"{}\"", query);
        println!("  小写: \"{}\"", query.to_lowercase());
        println!("  分词: {:?}", query.split_whitespace().collect::<Vec<_>>());

        match storage.query_bullets(query, 10).await {
            Ok(bullets) => {
                println!("  ✅ 结果: {} 条bullets", bullets.len());
                for (i, bullet) in bullets.iter().enumerate().take(2) {
                    println!(
                        "    {}. [tags: {:?}] {}",
                        i + 1,
                        bullet.tags,
                        bullet.content.chars().take(50).collect::<String>()
                    );
                }
            }
            Err(e) => {
                println!("  ❌ 错误: {}", e);
            }
        }
    }

    println!("\n===== 测试完成 =====");
    Ok(())
}
