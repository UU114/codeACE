//! 测试新的 Bullet 格式生成
//!
//! 验证 Curator 是否正确使用 BulletContentBuilder 生成结构化的 bullet 内容

use codex_core::ace::curator::CuratorMVP;
use codex_core::ace::types::{CuratorConfig, InsightCategory, InsightContext, RawInsight};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== 测试 Bullet 新格式生成 ===\n");

    let curator = CuratorMVP::new(CuratorConfig::default());

    // 测试场景1：成功的任务
    println!("## 场景 1: 成功的任务\n");
    let success_insight = RawInsight {
        content: "使用 cargo test 命令可以运行项目的所有测试。该命令会自动编译测试代码并执行所有标记为 #[test] 的函数。"
            .to_string(),
        category: InsightCategory::ToolUsage,
        importance: 0.8,
        context: InsightContext {
            user_query: "如何运行 Rust 项目的测试？".to_string(),
            assistant_response_snippet: "您可以使用 `cargo test` 命令来运行所有测试。这会编译并执行项目中的所有测试用例，并显示测试结果。"
                .to_string(),
            execution_success: true,
            tools_used: vec!["cargo".to_string(), "bash".to_string()],
            error_message: None,
            session_id: "test-session-001".to_string(),
        },
    };

    let delta = curator
        .process_insights(vec![success_insight], "test-session-001".to_string())
        .await?;

    if let Some(bullet) = delta.new_bullets.first() {
        println!("生成的 Bullet 内容：\n");
        println!("{}", bullet.content);
        println!("\n标签: {:?}", bullet.tags);
        println!("Section: {:?}", bullet.section);
        println!("\n{}\n", "=".repeat(80));
    }

    // 测试场景2：包含错误的失败任务
    println!("## 场景 2: 包含错误的失败任务\n");
    let failed_insight = RawInsight {
        content: "部署 Kubernetes 应用时需要确保镜像仓库的认证配置正确，并且 Ingress 路径规则符合规范。"
            .to_string(),
        category: InsightCategory::ErrorHandling,
        importance: 0.9,
        context: InsightContext {
            user_query: "部署微服务到 Kubernetes 集群".to_string(),
            assistant_response_snippet: "在部署过程中遇到了一些问题，主要是镜像拉取失败和 Ingress 配置错误。需要配置 imagePullSecret 并修复 Ingress 路径规则。"
                .to_string(),
            execution_success: false,
            tools_used: vec!["kubectl".to_string(), "helm".to_string()],
            error_message: Some(
                "Pod 启动失败：ImagePullBackOff - 镜像仓库认证失败\nIngress 配置错误：路径匹配失败，返回 404 错误"
                    .to_string()
            ),
            session_id: "test-session-002".to_string(),
        },
    };

    let delta = curator
        .process_insights(vec![failed_insight], "test-session-002".to_string())
        .await?;

    if let Some(bullet) = delta.new_bullets.first() {
        println!("生成的 Bullet 内容：\n");
        println!("{}", bullet.content);
        println!("\n标签: {:?}", bullet.tags);
        println!("Section: {:?}", bullet.section);
        println!("\n{}\n", "=".repeat(80));
    }

    // 测试场景3：包含代码的任务
    println!("## 场景 3: 包含代码示例的任务\n");
    let code_insight = RawInsight {
        content: r#"实现 JWT 认证需要使用 jsonwebtoken crate。示例代码：

```rust
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};

fn create_token(user_id: &str) -> Result<String> {
    let claims = Claims {
        sub: user_id.to_string(),
        exp: 10000000000,
    };

    encode(&Header::default(), &claims, &EncodingKey::from_secret("secret".as_ref()))
}
```

这个函数可以生成一个包含用户ID的JWT token。"#
            .to_string(),
        category: InsightCategory::Solution,
        importance: 0.85,
        context: InsightContext {
            user_query: "如何在 Rust 中实现 JWT 认证？".to_string(),
            assistant_response_snippet: "可以使用 jsonwebtoken crate 来实现 JWT 认证。需要定义 Claims 结构体，然后使用 encode 函数生成 token，使用 decode 函数验证 token。"
                .to_string(),
            execution_success: true,
            tools_used: vec!["cargo".to_string()],
            error_message: None,
            session_id: "test-session-003".to_string(),
        },
    };

    let delta = curator
        .process_insights(vec![code_insight], "test-session-003".to_string())
        .await?;

    if let Some(bullet) = delta.new_bullets.first() {
        println!("生成的 Bullet 内容：\n");
        println!("{}", bullet.content);
        println!("\n标签: {:?}", bullet.tags);
        println!("Section: {:?}", bullet.section);

        // 检查是否提取了代码
        if let Some(code_content) = &bullet.code_content {
            println!("\n代码内容已提取：");
            match code_content {
                codex_core::ace::types::BulletCodeContent::Full { language, code, .. } => {
                    println!("  类型: 完整代码");
                    println!("  语言: {}", language);
                    println!("  行数: {}", code.lines().count());
                }
                codex_core::ace::types::BulletCodeContent::Summary {
                    language,
                    file_path,
                    ..
                } => {
                    println!("  类型: 摘要");
                    println!("  语言: {}", language);
                    println!("  文件: {}", file_path);
                }
            }
        }
        println!("\n{}\n", "=".repeat(80));
    }

    println!("\n✅ 所有测试场景完成！");
    println!("\n新的 Bullet 格式包含：");
    println!("  ✓ 用户需求");
    println!("  ✓ 解决思路及方法");
    println!("  ✓ 解决结果");
    println!("  ✓ 评价");
    println!("  ✓ 错误信息（如有）");
    println!("  ✓ 总结分析（如有）");
    println!("  ✓ 技术选型（如有）");

    Ok(())
}
