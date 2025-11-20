//! Code Analyzer - Distinguish core code vs auxiliary code
//!
//! Determine storage strategy based on code length, complexity and other factors

use super::types::BulletCodeContent;

/// Code classification threshold (code below this line count is saved completely, code above this is summarized)
const CORE_CODE_LINE_THRESHOLD: usize = 100;

/// Code Analyzer
pub struct CodeAnalyzer {
    /// Core code line count threshold
    core_threshold: usize,
}

impl CodeAnalyzer {
    /// Create new code analyzer
    pub fn new() -> Self {
        Self {
            core_threshold: CORE_CODE_LINE_THRESHOLD,
        }
    }

    /// Create with custom threshold
    pub fn with_threshold(threshold: usize) -> Self {
        Self {
            core_threshold: threshold,
        }
    }

    /// Analyze code block and decide storage strategy
    ///
    /// # Parameters
    /// - `language`: Programming language
    /// - `code`: Code content
    /// - `file_path`: File path (optional)
    ///
    /// # Returns
    /// `BulletCodeContent` - Full save or summary save
    pub fn analyze_code(
        &self,
        language: &str,
        code: &str,
        file_path: Option<String>,
    ) -> BulletCodeContent {
        let line_count = code.lines().count();

        // Check if it's core code (below threshold)
        if line_count < self.core_threshold {
            // Save in full
            BulletCodeContent::Full {
                language: language.to_string(),
                code: code.to_string(),
                file_path,
            }
        } else {
            // Generate summary
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

    /// Generate code summary
    ///
    /// Extract function signatures, class definitions, important types, etc.
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

    /// Rust code summary
    fn summarize_rust(&self, code: &str) -> String {
        let mut summary = Vec::new();

        for line in code.lines() {
            let trimmed = line.trim();

            // Extract pub functions, structs, enums, traits
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
            format!("Rust code file ({} lines)", code.lines().count())
        } else {
            summary.join("\n")
        }
    }

    /// JavaScript/TypeScript code summary
    fn summarize_js_ts(&self, code: &str) -> String {
        let mut summary = Vec::new();

        for line in code.lines() {
            let trimmed = line.trim();

            // Extract functions, classes, interfaces, type definitions
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
                "JavaScript/TypeScript code file ({} lines)",
                code.lines().count()
            )
        } else {
            summary.join("\n")
        }
    }

    /// Python code summary
    fn summarize_python(&self, code: &str) -> String {
        let mut summary = Vec::new();

        for line in code.lines() {
            let trimmed = line.trim();

            // Extract function and class definitions
            if trimmed.starts_with("def ") || trimmed.starts_with("class ") {
                summary.push(line.to_string());
            }
        }

        if summary.is_empty() {
            format!("Python code file ({} lines)", code.lines().count())
        } else {
            summary.join("\n")
        }
    }

    /// Java code summary
    fn summarize_java(&self, code: &str) -> String {
        let mut summary = Vec::new();

        for line in code.lines() {
            let trimmed = line.trim();

            // Extract class, interface, method definitions
            if (trimmed.starts_with("public class ")
                || trimmed.starts_with("public interface ")
                || trimmed.starts_with("public ")
                || trimmed.starts_with("private ")
                || trimmed.starts_with("protected "))
                && (trimmed.contains('(')
                    || trimmed.contains("class ")
                    || trimmed.contains("interface "))
            {
                summary.push(line.to_string());
            }
        }

        if summary.is_empty() {
            format!("Java code file ({} lines)", code.lines().count())
        } else {
            summary.join("\n")
        }
    }

    /// Go code summary
    fn summarize_go(&self, code: &str) -> String {
        let mut summary = Vec::new();

        for line in code.lines() {
            let trimmed = line.trim();

            // Extract function, type, interface definitions
            if trimmed.starts_with("func ")
                || trimmed.starts_with("type ")
                || trimmed.starts_with("interface ")
            {
                summary.push(line.to_string());
            }
        }

        if summary.is_empty() {
            format!("Go code file ({} lines)", code.lines().count())
        } else {
            summary.join("\n")
        }
    }

    /// Generic code summary (simplified version)
    fn summarize_generic(&self, code: &str) -> String {
        let line_count = code.lines().count();
        let first_lines: Vec<&str> = code.lines().take(10).collect();

        format!(
            "Code file ({} lines)\n\nFirst few lines:\n{}",
            line_count,
            first_lines.join("\n")
        )
    }

    /// Extract key line ranges
    ///
    /// Identify line numbers of important code like function definitions, class definitions
    fn extract_key_lines(&self, language: &str, code: &str) -> Vec<(usize, usize)> {
        let mut key_lines = Vec::new();
        let mut current_start: Option<usize> = None;

        for (idx, line) in code.lines().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();

            // Detect key lines (based on language)
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
                // Empty line may indicate end of code block
                if let Some(start) = current_start {
                    key_lines.push((start, line_num - 1));
                    current_start = None;
                }
            }
        }

        // Handle last unclosed range
        if let Some(start) = current_start {
            key_lines.push((start, code.lines().count()));
        }

        key_lines
    }

    /// Check if should save in full
    ///
    /// In some cases, code should be saved in full even if it's long
    pub fn should_save_full(&self, language: &str, code: &str, file_path: Option<&str>) -> bool {
        let line_count = code.lines().count();

        // Below threshold, always save in full
        if line_count < self.core_threshold {
            return true;
        }

        // Config files always saved in full
        if let Some(path) = file_path
            && (path.ends_with(".json")
                || path.ends_with(".toml")
                || path.ends_with(".yaml")
                || path.ends_with(".yml")
                || path.ends_with(".config"))
        {
            return true;
        }

        // Some special language files (like SQL, Shell scripts) always saved in full
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
                // Expected result
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
                // Expected result
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

        // Config files should be saved in full even if long
        assert!(analyzer.should_save_full("json", &code, Some("config.json")));
    }
}
