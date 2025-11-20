// LAPS 系统集成测试
use chrono::Utc;
use codex_core::ace::background_optimizer::{BackgroundOptimizer, OptimizerConfig};
use codex_core::ace::content_classifier::{ContentClassifier, ContentType};
use codex_core::ace::knowledge_scope::{Context, Domain, KnowledgeScope, Language};
use codex_core::ace::lightweight_index::LightweightIndex;
use codex_core::ace::storage::BulletStorage;
use codex_core::ace::types::{Bullet, BulletMetadata, BulletSection, DeltaContext, Playbook};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 创建测试用的 Playbook
fn create_test_playbook(num_bullets: usize) -> Playbook {
    let mut playbook = Playbook::new();

    for i in 0..num_bullets {
        let bullet = Bullet {
            id: format!("test-bullet-{}", i),
            content: format!("这是测试 bullet {} 关于 Rust async/await 编程的内容", i),
            section: BulletSection::StrategiesAndRules,
            created_at: Utc::now() - chrono::Duration::days(i as i64),
            updated_at: Utc::now(),
            source_session_id: format!("session-{}", i),
            metadata: BulletMetadata::default(),
            tags: vec![],
            code_content: None,
        };

        playbook.add_bullet(bullet);
    }

    playbook
}

#[tokio::test]
async fn test_content_classification() {
    // 测试代码片段分类
    let code_snippet = r#"
```rust
async fn process_data() -> Result<()> {
    let data = fetch_data().await?;
    process(data).await
}
```
这是一个异步处理数据的代码示例
"#;

    let content_type = ContentClassifier::classify(code_snippet);
    assert!(matches!(content_type, ContentType::CodeSnippet));

    let (valid, reason) = ContentClassifier::validate_content(code_snippet);
    assert!(valid, "代码片段应该通过验证: {}", reason);
}

#[tokio::test]
async fn test_error_solution_classification() {
    let error_solution =
        "当遇到 'cannot borrow as mutable' 错误时，需要检查是否有多个可变引用同时存在";

    let content_type = ContentClassifier::classify(error_solution);
    assert!(matches!(content_type, ContentType::ErrorSolution));

    let (valid, _) = ContentClassifier::validate_content(error_solution);
    assert!(valid);
}

#[tokio::test]
async fn test_content_too_short() {
    let short_content = "太短了";

    let (valid, reason) = ContentClassifier::validate_content(short_content);
    assert!(!valid, "太短的内容应该被拒绝");
    assert!(reason.contains("太短") || reason.contains("简单"));
}

#[tokio::test]
async fn test_lightweight_index_build() {
    let playbook = create_test_playbook(50);

    // 构建索引
    let index = LightweightIndex::build_from_playbook(&playbook);

    // 验证索引构建成功
    assert!(index.size() > 0);
}

#[tokio::test]
async fn test_lightweight_index_search() {
    let playbook = create_test_playbook(100);
    let mut index = LightweightIndex::build_from_playbook(&playbook);

    // 测试搜索
    let results = index.search("Rust async", 10);

    assert!(!results.is_empty(), "应该找到相关的 bullets");
    assert!(results.len() <= 10, "结果数量应该不超过限制");

    // 验证结果相关性
    for bullet in results {
        let content_lower = bullet.content.to_lowercase();
        assert!(
            content_lower.contains("rust") || content_lower.contains("async"),
            "搜索结果应该包含查询关键词"
        );
    }
}

#[tokio::test]
async fn test_lightweight_index_incremental_update() {
    let mut playbook = create_test_playbook(10);
    let mut index = LightweightIndex::build_from_playbook(&playbook);

    let original_size = index.size();

    // 添加新 bullet
    let new_bullet = Bullet {
        id: "new-bullet".to_string(),
        content: "新添加的 bullet 关于 tokio".to_string(),
        section: BulletSection::StrategiesAndRules,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        source_session_id: "session-new".to_string(),
        metadata: BulletMetadata::default(),
        tags: vec![],
        code_content: None,
    };

    index.add_bullet(new_bullet.clone());

    assert_eq!(index.size(), original_size + 1);

    // 搜索新添加的 bullet
    let results = index.search("tokio", 5);
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_knowledge_scope_detection() {
    // 测试领域检测
    let web_content = "使用 HTTP REST API 构建 Web 服务";
    assert_eq!(KnowledgeScope::detect_domain(web_content), Domain::WebDev);

    let sys_content = "使用 async/await 处理并发和多线程";
    assert_eq!(
        KnowledgeScope::detect_domain(sys_content),
        Domain::SystemsProg
    );

    // 测试语言检测
    let rust_content = "使用 cargo build 编译项目";
    assert_eq!(
        KnowledgeScope::detect_language(rust_content),
        Language::Rust
    );

    let python_content = "使用 pip install 安装依赖";
    assert_eq!(
        KnowledgeScope::detect_language(python_content),
        Language::Python
    );
}

#[tokio::test]
async fn test_knowledge_scope_matching() {
    let scope =
        KnowledgeScope::new(Domain::WebDev, Language::Rust).with_project("my-web-app".to_string());

    // 完全匹配的上下文
    let perfect_context = Context {
        domain: Domain::WebDev,
        language: Language::Rust,
        project: Some("my-web-app".to_string()),
        query: "如何处理 HTTP 请求".to_string(),
    };

    // 部分匹配的上下文
    let partial_context = Context {
        domain: Domain::WebDev,
        language: Language::Python,
        project: None,
        query: "API 设计".to_string(),
    };

    // 不匹配的上下文
    let no_match_context = Context {
        domain: Domain::DataScience,
        language: Language::Python,
        project: None,
        query: "训练模型".to_string(),
    };

    let perfect_score = scope.match_score(&perfect_context);
    let partial_score = scope.match_score(&partial_context);
    let no_match_score = scope.match_score(&no_match_context);

    assert!(perfect_score > partial_score, "完全匹配应该得分更高");
    assert!(partial_score > no_match_score, "部分匹配应该比不匹配得分高");
}

#[tokio::test]
async fn test_background_optimizer_dedup() {
    let temp_dir = tempfile::tempdir().unwrap();
    let storage_path = temp_dir.path();

    let storage = BulletStorage::new(storage_path, 1000).unwrap();

    // 添加重复的 bullets
    let bullet1 = Bullet {
        id: "bullet-1".to_string(),
        content: "这是测试内容".to_string(),
        section: BulletSection::General,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        source_session_id: "session-1".to_string(),
        metadata: BulletMetadata::default(),
        tags: vec![],
        code_content: None,
    };

    let bullet2 = Bullet {
        id: "bullet-2".to_string(),
        content: "这是测试内容".to_string(), // 相同内容
        section: BulletSection::General,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        source_session_id: "session-2".to_string(),
        metadata: BulletMetadata::default(),
        tags: vec![],
        code_content: None,
    };

    let mut delta = DeltaContext::new("test-session".to_string());
    delta.new_bullets.push(bullet1);
    delta.new_bullets.push(bullet2);
    storage.merge_delta(delta).await.unwrap();

    let storage_arc = Arc::new(RwLock::new(storage));
    let optimizer = BackgroundOptimizer::new(storage_arc, OptimizerConfig::default());

    // 执行去重
    optimizer.optimize().await.unwrap();

    // 验证至少删除了一个重复项
    // 注意: optimize() 返回 Result<()>,不返回删除数量
    // 我们通过检查最终的 bullets 数量来验证
    let final_storage = optimizer.get_stats().await.unwrap();
    assert!(final_storage.total_bullets < 2, "去重后应该只剩一个 bullet");
}

#[tokio::test]
async fn test_background_optimizer_cleanup() {
    let temp_dir = tempfile::tempdir().unwrap();
    let storage_path = temp_dir.path();

    let storage = BulletStorage::new(storage_path, 1000).unwrap();

    // 添加一个从未被召回且很旧的 bullet
    let mut old_bullet = Bullet {
        id: "old-bullet".to_string(),
        content: "这是一个很旧的 bullet".to_string(),
        section: BulletSection::General,
        created_at: Utc::now() - chrono::Duration::days(35),
        updated_at: Utc::now(),
        source_session_id: "session-old".to_string(),
        metadata: BulletMetadata::default(),
        tags: vec![],
        code_content: None,
    };

    old_bullet.metadata.recall_count = 0;

    // 添加一个最近使用的 bullet
    let mut recent_bullet = Bullet {
        id: "recent-bullet".to_string(),
        content: "这是一个最近使用的 bullet，内容比较详细和有价值".to_string(),
        section: BulletSection::General,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        source_session_id: "session-recent".to_string(),
        metadata: BulletMetadata::default(),
        tags: vec![],
        code_content: None,
    };

    recent_bullet.metadata.recall_count = 5;
    recent_bullet.metadata.last_recall = Some(Utc::now() - chrono::Duration::days(2));
    recent_bullet.metadata.success_rate = 0.8;

    let mut delta = DeltaContext::new("test-session".to_string());
    delta.new_bullets.push(old_bullet);
    delta.new_bullets.push(recent_bullet);
    storage.merge_delta(delta).await.unwrap();

    let storage_arc = Arc::new(RwLock::new(storage));
    let optimizer = BackgroundOptimizer::new(storage_arc, OptimizerConfig::default());

    // 执行优化（包括清理）
    optimizer.optimize().await.unwrap();

    // 验证旧 bullet 被删除，新 bullet 保留
    let stats = optimizer.get_stats().await.unwrap();
    assert_eq!(stats.total_bullets, 1, "应该只保留一个有价值的 bullet");
}

#[tokio::test]
async fn test_optimizer_stats() {
    let temp_dir = tempfile::tempdir().unwrap();
    let storage_path = temp_dir.path();

    let storage = BulletStorage::new(storage_path, 1000).unwrap();

    // 创建不同类型的 bullets
    let mut delta = DeltaContext::new("test-session".to_string());

    for i in 0..20 {
        let mut bullet = Bullet {
            id: format!("bullet-{}", i),
            content: format!("测试内容 {} 关于编程技巧", i),
            section: BulletSection::General,
            created_at: Utc::now() - chrono::Duration::days((i % 40) as i64),
            updated_at: Utc::now(),
            source_session_id: format!("session-{}", i),
            metadata: BulletMetadata::default(),
            tags: vec![],
            code_content: None,
        };

        bullet.metadata.recall_count = i % 10;
        bullet.metadata.importance = 0.3 + ((i % 7) as f32 * 0.1);

        if i % 3 == 0 {
            bullet.metadata.last_recall = Some(Utc::now() - chrono::Duration::days(5));
        }

        delta.new_bullets.push(bullet);
    }

    storage.merge_delta(delta).await.unwrap();

    let storage_arc = Arc::new(RwLock::new(storage));
    let optimizer = BackgroundOptimizer::new(storage_arc, OptimizerConfig::default());

    let stats = optimizer.get_stats().await.unwrap();

    assert_eq!(stats.total_bullets, 20);
    assert!(stats.avg_recall >= 0.0);
    assert!(stats.bullets_last_week > 0 || stats.bullets_last_month > 0);

    // 打印统计信息
    println!("{}", stats.format());
}

#[tokio::test]
async fn test_dynamic_weight_calculation() {
    let mut metadata = BulletMetadata::default();
    metadata.importance = 0.5;
    metadata.recall_count = 0;

    let initial_weight = metadata.calculate_dynamic_weight();

    // 记录成功的召回
    for _ in 0..5 {
        metadata.record_recall("test context".to_string(), true);
    }

    let after_success_weight = metadata.calculate_dynamic_weight();

    assert!(
        after_success_weight > initial_weight,
        "成功召回后权重应该增加"
    );

    // 记录失败的召回
    for _ in 0..3 {
        metadata.record_recall("test context".to_string(), false);
    }

    let after_failure_weight = metadata.calculate_dynamic_weight();

    assert!(
        after_failure_weight < after_success_weight,
        "失败召回应该降低权重"
    );
}

#[tokio::test]
async fn test_full_laps_workflow() {
    // 完整的 LAPS 工作流测试
    let temp_dir = tempfile::tempdir().unwrap();
    let storage_path = temp_dir.path();

    // 1. 创建存储
    let storage = BulletStorage::new(storage_path, 1000).unwrap();

    // 2. 添加各种类型的内容
    let code_content = r#"
```rust
async fn handle_request() -> Result<Response> {
    let data = fetch_data().await?;
    Ok(Response::new(data))
}
```
异步处理 HTTP 请求的模式
"#;

    let bullet1 = Bullet {
        id: "bullet-code".to_string(),
        content: code_content.to_string(),
        section: BulletSection::CodeSnippetsAndTemplates,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        source_session_id: "session-1".to_string(),
        metadata: BulletMetadata::default(),
        tags: vec!["async".to_string(), "http".to_string()],
        code_content: None,
    };

    let error_content =
        "当遇到 'future cannot be sent between threads' 错误时，需要确保 Future 实现了 Send trait";

    let bullet2 = Bullet {
        id: "bullet-error".to_string(),
        content: error_content.to_string(),
        section: BulletSection::TroubleshootingAndPitfalls,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        source_session_id: "session-2".to_string(),
        metadata: BulletMetadata::default(),
        tags: vec!["async".to_string(), "error".to_string()],
        code_content: None,
    };

    // 通过 DeltaContext 添加 bullets
    let mut delta = DeltaContext::new("test-session".to_string());
    delta.new_bullets.push(bullet1);
    delta.new_bullets.push(bullet2);
    storage.merge_delta(delta).await.unwrap();

    // 3. 构建索引
    let playbook = storage.load_playbook().await.unwrap();
    let mut index = LightweightIndex::build_from_playbook(&playbook);

    // 4. 搜索相关内容
    let results = index.search("async 错误", 5);
    assert!(!results.is_empty(), "应该找到相关内容");

    // 5. 模拟使用并记录召回
    let mut playbook_for_update = storage.load_playbook().await.unwrap();
    for result in &results {
        if let Some(bullet_mut) = playbook_for_update.find_bullet_mut(&result.id) {
            bullet_mut
                .metadata
                .record_recall("async 错误处理".to_string(), true);
        }
    }
    storage.save_playbook(&playbook_for_update).await.unwrap();

    // 6. 创建优化器并执行优化
    let storage_arc = Arc::new(RwLock::new(storage));
    let optimizer = BackgroundOptimizer::new(storage_arc, OptimizerConfig::default());

    optimizer.optimize().await.unwrap();

    // 7. 检查统计
    let stats = optimizer.get_stats().await.unwrap();
    assert_eq!(stats.total_bullets, 2);
    assert!(stats.avg_recall > 0.0, "应该有召回记录");

    println!("✅ LAPS 完整工作流测试通过!");
    println!("{}", stats.format());
}
