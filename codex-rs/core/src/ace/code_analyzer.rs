//! 代码分析器 - 判断核心代码 vs 辅助代码
//!
//! 根据代码长度、复杂度等因素决定保存策略

use super::types::BulletCodeContent;

/// 代码分级阈值（小于此行数的代码完整保存，大于等于此行数的代码保存摘要）
const CORE_CODE_LINE_THRESHOLD: usize = 100;

/// 代码分析器
pub struct CodeAnalyzer {
    /// 核心代码行数阈值
    core_threshold: usize,
}

impl CodeAnalyzer {
    /// 创建新的代码分析器
    pub fn new() -> Self {
        Self {
            core_threshold: CORE_CODE_LINE_THRESHOLD,
        }
    }

    /// 使用自定义阈值创建
    pub fn with_threshold(threshold: usize) -> Self {
        Self {
            core_threshold: threshold,
        }
    }

    /// 分析代码块并决定保存策略
    ///
    /// # 参数
    /// - `language`: 编程语言
    /// - `code`: 代码内容
    /// - `file_path`: 文件路径（可选）
    ///
    /// # 返回
    /// `BulletCodeContent` - 完整保存或摘要保存
    pub fn analyze_code(
        &self,
        language: &str,
        code: &str,
        file_path: Option<String>,
    ) -> BulletCodeContent {
        let line_count = code.lines().count();

        // 判断是否为核心代码（小于阈值）
        if line_count < self.core_threshold {
            // 完整保存
            BulletCodeContent::Full {
                language: language.to_string(),
                code: code.to_string(),
                file_path,
            }
        } else {
            // 生成摘要
            let summary = self.generate_code_summary(language, code);
            let key_lines = self.extract_key_lines(language, code);

            BulletCodeContent::Summary {
                language: language.to_string(),
                summary,
                file_path: file_path.unwrap_or_else(|| "unknown".to_string()),
                key_lines: Some(key_lines),
            }
        }
    }

    /// 生成代码摘要
    ///
    /// 提取函数签名、类定义、重要类型等
    fn generate_code_summary(&self, language: &str, code: &str) -> String {
        match language.to_lowercase().as_str() {
            "rust" | "rs" => self.summarize_rust(code),
            "javascript" | "js" | "typescript" | "ts" => self.summarize_js_ts(code),
            "python" | "py" => self.summarize_python(code),
            "java" => self.summarize_java(code),
            "go" => self.summarize_go(code),
            _ => self.summarize_generic(code),
        }
    }

    /// Rust 代码摘要
    fn summarize_rust(&self, code: &str) -> String {
        let mut summary = Vec::new();

        for line in code.lines() {
            let trimmed = line.trim();

            // 提取 pub 函数、结构体、枚举、trait
            if trimmed.starts_with("pub fn ")
                || trimmed.starts_with("pub struct ")
                || trimmed.starts_with("pub enum ")
                || trimmed.starts_with("pub trait ")
                || trimmed.starts_with("impl ")
            {
                summary.push(line.to_string());
            }
        }

        if summary.is_empty() {
            format!("Rust 代码文件 ({} 行)", code.lines().count())
        } else {
            summary.join("\n")
        }
    }

    /// JavaScript/TypeScript 代码摘要
    fn summarize_js_ts(&self, code: &str) -> String {
        let mut summary = Vec::new();

        for line in code.lines() {
            let trimmed = line.trim();

            // 提取函数、类、接口、类型定义
            if trimmed.starts_with("export function ")
                || trimmed.starts_with("export class ")
                || trimmed.starts_with("export interface ")
                || trimmed.starts_with("export type ")
                || trimmed.starts_with("function ")
                || trimmed.starts_with("class ")
                || trimmed.starts_with("interface ")
                || trimmed.starts_with("type ")
            {
                summary.push(line.to_string());
            }
        }

        if summary.is_empty() {
            format!(
                "JavaScript/TypeScript 代码文件 ({} 行)",
                code.lines().count()
            )
        } else {
            summary.join("\n")
        }
    }

    /// Python 代码摘要
    fn summarize_python(&self, code: &str) -> String {
        let mut summary = Vec::new();

        for line in code.lines() {
            let trimmed = line.trim();

            // 提取函数和类定义
            if trimmed.starts_with("def ") || trimmed.starts_with("class ") {
                summary.push(line.to_string());
            }
        }

        if summary.is_empty() {
            format!("Python 代码文件 ({} 行)", code.lines().count())
        } else {
            summary.join("\n")
        }
    }

    /// Java 代码摘要
    fn summarize_java(&self, code: &str) -> String {
        let mut summary = Vec::new();

        for line in code.lines() {
            let trimmed = line.trim();

            // 提取类、接口、方法定义
            if trimmed.starts_with("public class ")
                || trimmed.starts_with("public interface ")
                || trimmed.starts_with("public ")
                || trimmed.starts_with("private ")
                || trimmed.starts_with("protected ")
            {
                if trimmed.contains('(')
                    || trimmed.contains("class ")
                    || trimmed.contains("interface ")
                {
                    summary.push(line.to_string());
                }
            }
        }

        if summary.is_empty() {
            format!("Java 代码文件 ({} 行)", code.lines().count())
        } else {
            summary.join("\n")
        }
    }

    /// Go 代码摘要
    fn summarize_go(&self, code: &str) -> String {
        let mut summary = Vec::new();

        for line in code.lines() {
            let trimmed = line.trim();

            // 提取函数、类型、接口定义
            if trimmed.starts_with("func ")
                || trimmed.starts_with("type ")
                || trimmed.starts_with("interface ")
            {
                summary.push(line.to_string());
            }
        }

        if summary.is_empty() {
            format!("Go 代码文件 ({} 行)", code.lines().count())
        } else {
            summary.join("\n")
        }
    }

    /// 通用代码摘要（简化版）
    fn summarize_generic(&self, code: &str) -> String {
        let line_count = code.lines().count();
        let first_lines: Vec<&str> = code.lines().take(10).collect();

        format!(
            "代码文件 ({} 行)\n\n前几行:\n{}",
            line_count,
            first_lines.join("\n")
        )
    }

    /// 提取关键行号范围
    ///
    /// 识别函数定义、类定义等重要代码的行号
    fn extract_key_lines(&self, language: &str, code: &str) -> Vec<(usize, usize)> {
        let mut key_lines = Vec::new();
        let mut current_start: Option<usize> = None;

        for (idx, line) in code.lines().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();

            // 检测关键行（根据语言）
            let is_key_line = match language.to_lowercase().as_str() {
                "rust" | "rs" => {
                    trimmed.starts_with("pub fn ")
                        || trimmed.starts_with("pub struct ")
                        || trimmed.starts_with("impl ")
                }
                "javascript" | "js" | "typescript" | "ts" => {
                    trimmed.starts_with("export function ")
                        || trimmed.starts_with("export class ")
                        || trimmed.starts_with("function ")
                        || trimmed.starts_with("class ")
                }
                "python" | "py" => trimmed.starts_with("def ") || trimmed.starts_with("class "),
                _ => false,
            };

            if is_key_line {
                if current_start.is_none() {
                    current_start = Some(line_num);
                }
            } else if trimmed.is_empty() && current_start.is_some() {
                // 空行可能表示代码块结束
                if let Some(start) = current_start {
                    key_lines.push((start, line_num - 1));
                    current_start = None;
                }
            }
        }

        // 处理最后一个未关闭的范围
        if let Some(start) = current_start {
            key_lines.push((start, code.lines().count()));
        }

        key_lines
    }

    /// 判断是否应该完整保存
    ///
    /// 某些情况下即使代码很长也应该完整保存
    pub fn should_save_full(&self, language: &str, code: &str, file_path: Option<&str>) -> bool {
        let line_count = code.lines().count();

        // 小于阈值，总是完整保存
        if line_count < self.core_threshold {
            return true;
        }

        // 配置文件总是完整保存
        if let Some(path) = file_path {
            if path.ends_with(".json")
                || path.ends_with(".toml")
                || path.ends_with(".yaml")
                || path.ends_with(".yml")
                || path.ends_with(".config")
            {
                return true;
            }
        }

        // 某些特殊语言文件（如 SQL、Shell 脚本）总是完整保存
        matches!(
            language.to_lowercase().as_str(),
            "sql" | "bash" | "sh" | "shell"
        )
    }
}

impl Default for CodeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_code_full_save() {
        let analyzer = CodeAnalyzer::new();
        let code = "fn main() {\n    println!(\"Hello\");\n}";

        let result = analyzer.analyze_code("rust", code, None);

        match result {
            BulletCodeContent::Full { .. } => {
                // 预期结果
            }
            BulletCodeContent::Summary { .. } => {
                panic!("Small code should be saved in full");
            }
        }
    }

    #[test]
    fn test_large_code_summary() {
        let analyzer = CodeAnalyzer::new();
        let code = (0..250)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");

        let result = analyzer.analyze_code("rust", &code, Some("test.rs".to_string()));

        match result {
            BulletCodeContent::Summary { .. } => {
                // 预期结果
            }
            BulletCodeContent::Full { .. } => {
                panic!("Large code should be summarized");
            }
        }
    }

    #[test]
    fn test_rust_summary() {
        let analyzer = CodeAnalyzer::new();
        let code = r#"
pub struct MyStruct {
    field: String,
}

impl MyStruct {
    pub fn new() -> Self {
        Self { field: String::new() }
    }
}

fn helper() {
    // private function
}
"#;

        let summary = analyzer.summarize_rust(code);
        assert!(summary.contains("pub struct MyStruct"));
        assert!(summary.contains("impl MyStruct"));
    }

    #[test]
    fn test_config_file_full_save() {
        let analyzer = CodeAnalyzer::new();
        let code = (0..250)
            .map(|_| "config line")
            .collect::<Vec<_>>()
            .join("\n");

        // 配置文件应该完整保存，即使很长
        assert!(analyzer.should_save_full("json", &code, Some("config.json")));
    }
}
