//! ACE 集成测试 - 模拟多轮对话生成 Playbook
//!
//! 测试完整的 Mission → TodoList → Reflector → Curator → Storage 工作流

#[cfg(feature = "ace")]
#[cfg(test)]
mod ace_integration_tests {
    use codex_core::ace::ACEPlugin;
    use codex_core::ace::ReflectorMVP;
    use codex_core::ace::types::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    /// 模拟对话场景
    struct ConversationScenario {
        user_query: String,
        assistant_response: String,
        execution_result: ExecutionResult,
    }

    #[tokio::test]
    async fn test_multiple_conversations_generate_playbook() {
        // 创建临时目录
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().to_path_buf();

        println!("\n📁 测试存储路径: {:?}\n", storage_path);

        // 创建 ACE 配置
        let config = ACEConfig {
            enabled: true,
            storage_path: storage_path.to_str().unwrap().to_string(),
            max_entries: 500,
            reflector: ReflectorConfig::default(),
            context: ContextConfig::default(),
        };

        // 初始化 ACE Plugin
        let ace_plugin = ACEPlugin::new(config).unwrap();

        // 场景 1: Rust 项目测试
        println!("🧪 场景 1: Rust 项目测试");
        let scenario1 = ConversationScenario {
            user_query: "运行项目的所有测试".to_string(),
            assistant_response: "我将使用 `cargo test` 运行所有测试。这会编译并执行所有测试用例。"
                .to_string(),
            execution_result: ExecutionResult {
                success: true,
                tools_used: vec!["bash".to_string()],
                output: Some("running 42 tests\ntest result: ok. 42 passed".to_string()),
                ..Default::default()
            },
        };

        ace_plugin.on_todo_completed(
            "运行测试".to_string(),
            format!(
                "用户: {}\n助手: {}",
                scenario1.user_query, scenario1.assistant_response
            ),
            "session-1".to_string(),
        );

        // 场景 2: TypeScript 项目构建错误修复
        println!("🔧 场景 2: TypeScript 构建错误修复");
        let scenario2 = ConversationScenario {
            user_query: "修复 TypeScript 编译错误".to_string(),
            assistant_response: r#"发现类型错误。我将修复 `src/utils.ts` 中的类型定义：

```typescript
// 修复前
function processData(data) {
  return data.map(x => x.value);
}

// 修复后
function processData(data: Array<{value: number}>): number[] {
  return data.map(x => x.value);
}
```

这个修复添加了明确的类型注解。"#
                .to_string(),
            execution_result: ExecutionResult {
                success: true,
                tools_used: vec!["edit".to_string()],
                output: Some("File updated successfully".to_string()),
                ..Default::default()
            },
        };

        ace_plugin.on_todo_completed(
            "修复类型错误".to_string(),
            format!(
                "用户: {}\n助手: {}",
                scenario2.user_query, scenario2.assistant_response
            ),
            "session-2".to_string(),
        );

        // 场景 3: Python 包安装
        println!("📦 场景 3: Python 包安装");
        let scenario3 = ConversationScenario {
            user_query: "安装项目依赖".to_string(),
            assistant_response: "我将使用 pip 安装 requirements.txt 中的依赖。选择 pip 因为这是标准的 Python 包管理工具。"
                .to_string(),
            execution_result: ExecutionResult {
                success: true,
                tools_used: vec!["bash".to_string()],
                output: Some("Successfully installed 15 packages".to_string()),
                ..Default::default()
            },
        };

        ace_plugin.on_todo_completed(
            "安装依赖".to_string(),
            format!(
                "用户: {}\n助手: {}",
                scenario3.user_query, scenario3.assistant_response
            ),
            "session-3".to_string(),
        );

        // 场景 4: Git 操作
        println!("🌿 场景 4: Git 提交");
        let scenario4 = ConversationScenario {
            user_query: "提交代码变更".to_string(),
            assistant_response: r#"我将创建一个提交：

1. 使用 `git add .` 暂存所有更改
2. 使用 `git commit -m "feat: add user authentication"` 提交

选择 conventional commits 格式因为它提供了清晰的提交历史。"#
                .to_string(),
            execution_result: ExecutionResult {
                success: true,
                tools_used: vec!["bash".to_string()],
                output: Some("[main abc1234] feat: add user authentication".to_string()),
                ..Default::default()
            },
        };

        ace_plugin.on_todo_completed(
            "提交代码".to_string(),
            format!(
                "用户: {}\n助手: {}",
                scenario4.user_query, scenario4.assistant_response
            ),
            "session-4".to_string(),
        );

        // 场景 5: API 调用错误处理
        println!("🚨 场景 5: API 错误处理");
        let scenario5 = ConversationScenario {
            user_query: "修复 API 调用失败问题".to_string(),
            assistant_response: r#"分析错误后，问题是缺少错误处理。添加 try-catch：

```javascript
async function fetchUserData(userId) {
  try {
    const response = await fetch(`/api/users/${userId}`);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    console.error('Failed to fetch user:', error);
    return null;
  }
}
```

这样可以优雅地处理网络错误。"#
                .to_string(),
            execution_result: ExecutionResult {
                success: false,
                tools_used: vec!["edit".to_string()],
                error: Some("Network timeout".to_string()),
                errors: vec!["Connection refused".to_string()],
                ..Default::default()
            },
        };

        ace_plugin.on_todo_completed(
            "添加错误处理".to_string(),
            format!(
                "用户: {}\n助手: {}",
                scenario5.user_query, scenario5.assistant_response
            ),
            "session-5".to_string(),
        );

        // 场景 6: Docker 容器配置
        println!("🐳 场景 6: Docker 配置");
        let scenario6 = ConversationScenario {
            user_query: "创建 Docker 配置".to_string(),
            assistant_response: r#"创建 Dockerfile 用于 Node.js 应用：

```dockerfile
FROM node:18-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production
COPY . .
EXPOSE 3000
CMD ["node", "server.js"]
```

选择 alpine 镜像因为体积小，使用 npm ci 确保可重现构建。"#
                .to_string(),
            execution_result: ExecutionResult {
                success: true,
                tools_used: vec!["write".to_string()],
                output: Some("Dockerfile created".to_string()),
                ..Default::default()
            },
        };

        ace_plugin.on_todo_completed(
            "创建 Dockerfile".to_string(),
            format!(
                "用户: {}\n助手: {}",
                scenario6.user_query, scenario6.assistant_response
            ),
            "session-6".to_string(),
        );

        // 等待异步处理完成
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // 加载并检查 playbook
        println!("\n📖 检查生成的 Playbook\n");
        let playbook_path = storage_path.join("playbook.json");
        assert!(
            playbook_path.exists(),
            "Playbook 文件应该已创建: {:?}",
            playbook_path
        );

        // 读取 playbook
        let playbook_content = std::fs::read_to_string(&playbook_path).unwrap();
        let playbook: serde_json::Value = serde_json::from_str(&playbook_content).unwrap();

        println!("✅ Playbook 版本: {}", playbook["version"]);
        println!(
            "✅ 总 Bullets 数: {}",
            playbook["metadata"]["total_bullets"]
        );
        println!(
            "✅ 分类统计: {}",
            serde_json::to_string_pretty(&playbook["metadata"]["section_counts"]).unwrap()
        );

        // 验证至少有一些 bullets 被创建
        let total_bullets = playbook["metadata"]["total_bullets"].as_u64().unwrap();
        assert!(
            total_bullets > 0,
            "应该生成了至少一些 bullets，实际: {}",
            total_bullets
        );

        // 打印部分 bullets 示例
        println!("\n📝 Bullets 示例:\n");
        for (section, bullets) in playbook["bullets"].as_object().unwrap() {
            if let Some(bullets_array) = bullets.as_array() {
                println!("📂 {} ({} 条)", section, bullets_array.len());
                for bullet in bullets_array.iter().take(2) {
                    println!("  • {}", bullet["content"]);
                    println!(
                        "    标签: {:?}",
                        bullet["tags"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|t| t.as_str().unwrap())
                            .collect::<Vec<_>>()
                    );
                }
                if bullets_array.len() > 2 {
                    println!("  ... 还有 {} 条", bullets_array.len() - 2);
                }
                println!();
            }
        }

        // 验证不同分类
        let bullets_obj = playbook["bullets"].as_object().unwrap();
        println!(
            "✅ 生成的分类: {:?}\n",
            bullets_obj.keys().collect::<Vec<_>>()
        );

        // 最后，将 playbook 复制到用户目录供查看
        let user_playbook = PathBuf::from("/home/com/codeACE/codex-rs/test20251114/playbook.json");
        std::fs::create_dir_all(user_playbook.parent().unwrap()).ok();
        std::fs::copy(&playbook_path, &user_playbook).ok();

        println!("💾 Playbook 已保存到: {:?}", user_playbook);
        println!("\n✨ 集成测试完成！");
    }

    #[tokio::test]
    async fn test_reflector_directly_generates_insights() {
        println!("\n🔬 直接测试 Reflector 生成 Insights\n");

        let reflector = ReflectorMVP::new(codex_core::ace::reflector::ReflectorConfig::default());

        // 测试场景：安装依赖并运行测试
        let user_query = "设置项目并运行测试";
        let assistant_response = r#"我将执行以下步骤：

1. 使用 `npm install` 安装依赖（选择 npm 因为这是 Node.js 标准工具）
2. 使用 `npm test` 运行测试

让我开始执行：

```bash
npm install
npm test
```

测试执行成功！"#;

        let execution_result = ExecutionResult {
            success: true,
            tools_used: vec!["bash".to_string()],
            output: Some("15 tests passed".to_string()),
            ..Default::default()
        };

        let insights = reflector
            .analyze_conversation(
                user_query,
                assistant_response,
                &execution_result,
                "test-session".to_string(),
            )
            .await
            .unwrap();

        println!("📊 生成的 Insights 数量: {}", insights.len());
        for (i, insight) in insights.iter().enumerate() {
            println!("\nInsight #{}", i + 1);
            println!("  类别: {:?}", insight.category);
            println!("  重要性: {:.2}", insight.importance);
            println!("  内容: {}", insight.content);
            println!("  工具: {:?}", insight.context.tools_used);
        }

        assert!(!insights.is_empty(), "应该生成至少一个 insight");

        // 验证不同类型的 insights
        let has_tool_usage = insights
            .iter()
            .any(|i| matches!(i.category, InsightCategory::ToolUsage));
        let has_pattern = insights
            .iter()
            .any(|i| matches!(i.category, InsightCategory::Pattern));

        println!("\n✅ 包含工具使用 insight: {}", has_tool_usage);
        println!("✅ 包含模式 insight: {}", has_pattern);
    }
}
