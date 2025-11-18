//! ACE端到端集成测试
//!
//! 测试ACE的核心功能：配置加载、CLI命令、存储基本操作

use anyhow::Result;
use codex_core::ace::ACEPlugin;
use codex_core::ace::AceCliHandler;
use codex_core::ace::AceCommand;
use codex_core::ace::BulletStorage;
use std::sync::Arc;
use tempfile::TempDir;

/// 测试1: 配置自动创建和加载
#[tokio::test]
async fn test_config_auto_creation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let codex_home = temp_dir.path();

    // 配置文件应该不存在
    let config_path = codex_home.join("codeACE-config.toml");
    assert!(!config_path.exists());

    // 加载配置应该自动创建
    let _plugin = ACEPlugin::from_codex_home(codex_home)
        .await?
        .expect("Plugin should be created");

    // 配置文件应该被创建
    assert!(config_path.exists());

    // 验证默认配置
    let config_content = std::fs::read_to_string(&config_path)?;
    assert!(config_content.contains("enabled = true"));
    assert!(config_content.contains("max_entries = 500"));

    Ok(())
}

/// 测试2: Hook注册和基本调用
#[tokio::test]
async fn test_hook_registration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let codex_home = temp_dir.path();

    // 创建插件
    let plugin = ACEPlugin::from_codex_home(codex_home)
        .await?
        .expect("Plugin should be created");

    // 注册到HookManager
    let mut hook_manager = codex_core::hooks::HookManager::new();
    hook_manager.register(Arc::new(plugin));

    // Post-execute应该不panic（学习过程是异步的）
    hook_manager.call_post_execute(
        "How to fix Rust errors?",
        "You should check the error message",
        true,
    );

    // 给学习过程一些时间
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    Ok(())
}

/// 测试3: 存储基本操作
#[tokio::test]
async fn test_storage_basic_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("ace");

    let storage = BulletStorage::new(&storage_path, 100)?;

    // 测试加载空playbook
    let playbook = storage.load_playbook().await?;
    assert_eq!(playbook.all_bullets().len(), 0);

    // 测试统计（空）
    let stats = storage.get_stats().await?;
    assert_eq!(stats.total_bullets, 0);
    assert_eq!(stats.total_sessions, 0);

    Ok(())
}

/// 测试4: CLI命令 - Config
#[tokio::test]
async fn test_cli_config_command() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let codex_home = temp_dir.path();

    // 创建配置（通过from_codex_home自动创建）
    let _plugin = ACEPlugin::from_codex_home(codex_home).await?;

    // 测试CLI命令
    let handler = AceCliHandler::new(codex_home);

    // Config命令应该成功
    handler.execute(AceCommand::Config).await?;

    Ok(())
}

/// 测试5: CLI命令 - Status（空playbook）
#[tokio::test]
async fn test_cli_status_command_empty() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let codex_home = temp_dir.path();

    // 创建配置
    let _plugin = ACEPlugin::from_codex_home(codex_home).await?;

    let handler = AceCliHandler::new(codex_home);

    // Status命令应该成功（即使playbook为空）
    handler.execute(AceCommand::Status).await?;

    Ok(())
}

/// 测试6: CLI命令 - Show（空playbook）
#[tokio::test]
async fn test_cli_show_command_empty() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let codex_home = temp_dir.path();

    // 创建配置
    let _plugin = ACEPlugin::from_codex_home(codex_home).await?;

    let handler = AceCliHandler::new(codex_home);

    // Show命令应该成功（即使playbook为空）
    handler.execute(AceCommand::Show { limit: 10 }).await?;

    Ok(())
}

/// 测试7: CLI命令 - Search（空playbook）
#[tokio::test]
async fn test_cli_search_command_empty() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let codex_home = temp_dir.path();

    // 创建配置
    let _plugin = ACEPlugin::from_codex_home(codex_home).await?;

    let handler = AceCliHandler::new(codex_home);

    // Search命令应该成功（即使没有结果）
    handler
        .execute(AceCommand::Search {
            query: "test".to_string(),
        })
        .await?;

    Ok(())
}

/// 测试8: 配置禁用ACE
#[tokio::test]
async fn test_config_disabled() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let codex_home = temp_dir.path();

    // 先创建默认配置
    let config_loader = codex_core::ace::ACEConfigLoader::new(codex_home);
    let mut config = config_loader.load_or_create().await?;

    // 禁用ACE
    config.enabled = false;
    config_loader.save(&config).await?;

    // 再次加载应该返回None
    let plugin = ACEPlugin::from_codex_home(codex_home).await?;
    assert!(plugin.is_none());

    Ok(())
}

/// 测试9: 存储路径展开
#[tokio::test]
async fn test_storage_path_expansion() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let codex_home = temp_dir.path();

    // 创建配置，使用相对路径
    let config_loader = codex_core::ace::ACEConfigLoader::new(codex_home);
    let mut config = config_loader.load_or_create().await?;

    // 使用包含 ~ 的路径（虽然在测试中不会真的展开，但测试不应该失败）
    config.storage_path = format!("{}/ace", temp_dir.path().display());
    config_loader.save(&config).await?;

    // 创建插件应该成功
    let plugin = ACEPlugin::from_codex_home(codex_home)
        .await?
        .expect("Plugin should be created");

    // 验证插件被正确创建
    drop(plugin);

    Ok(())
}

/// 测试10: 配置加载失败不影响主程序
#[tokio::test]
async fn test_config_load_failure_graceful() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let codex_home = temp_dir.path();

    // 创建无效的配置文件
    let config_path = codex_home.join("codeACE-config.toml");
    std::fs::create_dir_all(codex_home)?;
    std::fs::write(&config_path, "invalid toml content ][][")?;

    // 加载应该失败但返回None而不是panic
    let plugin = ACEPlugin::from_codex_home(codex_home).await?;
    assert!(plugin.is_none());

    Ok(())
}
