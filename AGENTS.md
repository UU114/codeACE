# Rust/codex-rs

In the codex-rs folder where the rust code lives:

- Crate names are prefixed with `codex-`. For example, the `core` folder's crate is named `codex-core`
- When using format! and you can inline variables into {}, always do that.
- Install any commands the repo relies on (for example `just`, `rg`, or `cargo-insta`) if they aren't already available before running instructions here.
- Never add or modify any code related to `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` or `CODEX_SANDBOX_ENV_VAR`.
  - You operate in a sandbox where `CODEX_SANDBOX_NETWORK_DISABLED=1` will be set whenever you use the `shell` tool. Any existing code that uses `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` was authored with this fact in mind. It is often used to early exit out of tests that the author knew you would not be able to run given your sandbox limitations.
  - Similarly, when you spawn a process using Seatbelt (`/usr/bin/sandbox-exec`), `CODEX_SANDBOX=seatbelt` will be set on the child process. Integration tests that want to run Seatbelt themselves cannot be run under Seatbelt, so checks for `CODEX_SANDBOX=seatbelt` are also often used to early exit out of tests, as appropriate.
- Always collapse if statements per https://rust-lang.github.io/rust-clippy/master/index.html#collapsible_if
- Always inline format! args when possible per https://rust-lang.github.io/rust-clippy/master/index.html#uninlined_format_args
- Use method references over closures when possible per https://rust-lang.github.io/rust-clippy/master/index.html#redundant_closure_for_method_calls
- Do not use unsigned integer even if the number cannot be negative.
- When writing tests, prefer comparing the equality of entire objects over fields one by one.
- When making a change that adds or changes an API, ensure that the documentation in the `docs/` folder is up to date if applicable.

Run `just fmt` (in `codex-rs` directory) automatically after making Rust code changes; do not ask for approval to run it. Before finalizing a change to `codex-rs`, run `just fix -p <project>` (in `codex-rs` directory) to fix any linter issues in the code. Prefer scoping with `-p` to avoid slow workspace‑wide Clippy builds; only run `just fix` without `-p` if you changed shared crates. Additionally, run the tests:

1. Run the test for the specific project that was changed. For example, if changes were made in `codex-rs/tui`, run `cargo test -p codex-tui`.
2. Once those pass, if any changes were made in common, core, or protocol, run the complete test suite with `cargo test --all-features`.
   When running interactively, ask the user before running `just fix` to finalize. `just fmt` does not require approval. project-specific or individual tests can be run without asking the user, but do ask the user before running the complete test suite.

## TUI style conventions

See `codex-rs/tui/styles.md`.

## TUI code conventions

- Use concise styling helpers from ratatui’s Stylize trait.
  - Basic spans: use "text".into()
  - Styled spans: use "text".red(), "text".green(), "text".magenta(), "text".dim(), etc.
  - Prefer these over constructing styles with `Span::styled` and `Style` directly.
  - Example: patch summary file lines
    - Desired: vec!["  └ ".into(), "M".red(), " ".dim(), "tui/src/app.rs".dim()]

### TUI Styling (ratatui)

- Prefer Stylize helpers: use "text".dim(), .bold(), .cyan(), .italic(), .underlined() instead of manual Style where possible.
- Prefer simple conversions: use "text".into() for spans and vec![…].into() for lines; when inference is ambiguous (e.g., Paragraph::new/Cell::from), use Line::from(spans) or Span::from(text).
- Computed styles: if the Style is computed at runtime, using `Span::styled` is OK (`Span::from(text).set_style(style)` is also acceptable).
- Avoid hardcoded white: do not use `.white()`; prefer the default foreground (no color).
- Chaining: combine helpers by chaining for readability (e.g., url.cyan().underlined()).
- Single items: prefer "text".into(); use Line::from(text) or Span::from(text) only when the target type isn’t obvious from context, or when using .into() would require extra type annotations.
- Building lines: use vec![…].into() to construct a Line when the target type is obvious and no extra type annotations are needed; otherwise use Line::from(vec![…]).
- Avoid churn: don’t refactor between equivalent forms (Span::styled ↔ set_style, Line::from ↔ .into()) without a clear readability or functional gain; follow file‑local conventions and do not introduce type annotations solely to satisfy .into().
- Compactness: prefer the form that stays on one line after rustfmt; if only one of Line::from(vec![…]) or vec![…].into() avoids wrapping, choose that. If both wrap, pick the one with fewer wrapped lines.

### Text wrapping

- Always use textwrap::wrap to wrap plain strings.
- If you have a ratatui Line and you want to wrap it, use the helpers in tui/src/wrapping.rs, e.g. word_wrap_lines / word_wrap_line.
- If you need to indent wrapped lines, use the initial_indent / subsequent_indent options from RtOptions if you can, rather than writing custom logic.
- If you have a list of lines and you need to prefix them all with some prefix (optionally different on the first vs subsequent lines), use the `prefix_lines` helper from line_utils.

## Tests

### Snapshot tests

This repo uses snapshot tests (via `insta`), especially in `codex-rs/tui`, to validate rendered output. When UI or text output changes intentionally, update the snapshots as follows:

- Run tests to generate any updated snapshots:
  - `cargo test -p codex-tui`
- Check what’s pending:
  - `cargo insta pending-snapshots -p codex-tui`
- Review changes by reading the generated `*.snap.new` files directly in the repo, or preview a specific file:
  - `cargo insta show -p codex-tui path/to/file.snap.new`
- Only if you intend to accept all new snapshots in this crate, run:
  - `cargo insta accept -p codex-tui`

If you don’t have the tool:

- `cargo install cargo-insta`

### Test assertions

- Tests should use pretty_assertions::assert_eq for clearer diffs. Import this at the top of the test module if it isn't already.

### Integration tests (core)

- Prefer the utilities in `core_test_support::responses` when writing end-to-end Codex tests.

- All `mount_sse*` helpers return a `ResponseMock`; hold onto it so you can assert against outbound `/responses` POST bodies.
- Use `ResponseMock::single_request()` when a test should only issue one POST, or `ResponseMock::requests()` to inspect every captured `ResponsesRequest`.
- `ResponsesRequest` exposes helpers (`body_json`, `input`, `function_call_output`, `custom_tool_call_output`, `call_output`, `header`, `path`, `query_param`) so assertions can target structured payloads instead of manual JSON digging.
- Build SSE payloads with the provided `ev_*` constructors and the `sse(...)`.

- Typical pattern:

  ```rust
  let mock = responses::mount_sse_once(&server, responses::sse(vec![
      responses::ev_response_created("resp-1"),
      responses::ev_function_call(call_id, "shell", &serde_json::to_string(&args)?),
      responses::ev_completed("resp-1"),
  ])).await;

  codex.submit(Op::UserTurn { ... }).await?;

  // Assert request body if needed.
  let request = mock.single_request();
  // assert using request.function_call_output(call_id) or request.json_body() or other helpers.
  ```

## 已知问题和修复历史

### 任务链中断问题 (2025-01)

**问题描述**：
在某些情况下，AI 返回非工具调用的响应（如纯文本消息）后，任务会直接退出，导致无法继续多轮对话。这在 read-only sandbox 模式下尤其明显。

**根本原因**：
在 `codex-rs/core/src/codex.rs` 的主任务循环中，当 `process_items()` 返回空的 `responses` 列表时（即没有待执行的工具调用），代码会直接 `break` 退出循环，即使有其他内容（如 AI 消息）已被记录和处理。

**修复方案**：
1. **core/src/codex.rs (1916-1972行)**：改进空响应处理逻辑
   - 检查 `items_to_record_in_conversation_history` 是否为空
   - 如果有内容被记录但没有工具调用，发送 BackgroundEvent 通知客户端
   - 继续循环等待用户输入，而不是直接退出

2. **core/src/response_processing.rs**：增强错误处理和诊断
   - 为不匹配的 ResponseItem 类型记录详细的诊断信息
   - 发送 BackgroundEvent 通知客户端跳过的项
   - 统计并报告跳过的项数量

3. **core/src/safety.rs**：改进 read-only sandbox 反馈
   - 在 `assess_patch_safety()` 中提前检测 read-only 策略
   - 返回明确的错误消息，指导用户切换 sandbox 模式

**预防措施**：
- 在处理响应流时，始终区分"没有内容"和"没有待执行的工具调用"
- 为所有可能的 ResponseItem 类型提供明确的处理路径
- 在 read-only 模式下，提前验证操作的可行性

## 功能增强历史

### LLM 通信日志记录 (2025-11-18)

**功能描述**：
在 debug 编译模式下，自动记录所有与 LLM 的通信日志，包括请求和响应的完整原始数据。

**实现细节**：
1. **新增模块**：`codex-rs/core/src/llm_logger.rs`
   - 使用 `#[cfg(debug_assertions)]` 条件编译，仅在 debug 模式下启用
   - 采用 JSON Lines (JSONL) 格式，每天一个日志文件
   - 日志文件路径：`~/.codeACE/debug_logs/llm_YYYY-MM-DD.jsonl`
   - 异步写入，不阻塞主流程

2. **集成点**：
   - **Responses API** (`core/src/client.rs`):
     - 第 354 行：记录请求数据
     - 第 770-775 行：在 `process_sse()` 中记录每个 SSE 响应事件
   - **Chat Completions API** (`core/src/chat_completions.rs`):
     - 第 377-380 行：记录请求数据
     - 第 591-594 行：在 `process_chat_sse()` 中记录每个 SSE 响应事件

3. **日志格式**：
   ```json
   {
     "timestamp": "2025-11-18T13:03:03.907251071+00:00",
     "type": "request",
     "api": "responses_api",
     "request_id": "test-id-123",
     "data": { ... }
   }
   ```

4. **依赖变更**：
   - 在 `codex-rs/core/Cargo.toml` 中添加了 `lazy_static` 依赖

**使用说明**：
- Debug 编译：`cargo build`（默认）或 `cargo build --profile dev`
- Release 编译时，所有日志记录代码会被完全移除，零性能开销
- 详细文档：见 `ref/llm-logger/README.md`

**注意事项**：
- 日志文件可能包含敏感信息，请妥善保管
- 建议定期清理旧日志文件以节省磁盘空间
- 在高频请求场景下可能有轻微性能影响
