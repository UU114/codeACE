// ! Bullet内容格式化工具
//!
//! 提供结构化的bullet内容格式，确保每条学习记录都包含完整信息。

use std::fmt::Write;

/// Bullet内容构建器
///
/// 按照标准格式构建bullet内容，包含：
/// - 必须项：用户需求、解决思路、解决结果、评价
/// - 可选项：需求分析、错误信息、总结分析、实施方案
/// - 代码处理：<=100行完整保存，>100行摘要+位置
pub struct BulletContentBuilder {
    /// 用户需求（必须）
    user_requirement: Option<String>,

    /// 需求分析（可选）
    requirement_analysis: Option<String>,

    /// 解决思路及方法（必须）
    solution_approach: Option<String>,

    /// 遇到的错误（可选）
    errors_encountered: Option<Vec<String>>,

    /// 解决结果（必须）
    solution_result: Option<String>,

    /// 总结分析（可选）
    summary_analysis: Option<String>,

    /// 实施方案（可选）
    implementation_plan: Option<String>,

    /// 方案文件位置（如果方案很长）
    plan_file_path: Option<String>,

    /// 评价（必须）
    evaluation: Option<String>,

    /// 关键决策（可选）
    key_decisions: Option<Vec<String>>,

    /// 技术选型（可选）
    tech_stack: Option<Vec<String>>,
}

impl BulletContentBuilder {
    pub fn new() -> Self {
        Self {
            user_requirement: None,
            requirement_analysis: None,
            solution_approach: None,
            errors_encountered: None,
            solution_result: None,
            summary_analysis: None,
            implementation_plan: None,
            plan_file_path: None,
            evaluation: None,
            key_decisions: None,
            tech_stack: None,
        }
    }

    /// 设置用户需求（必须）
    pub fn user_requirement(mut self, req: impl Into<String>) -> Self {
        self.user_requirement = Some(req.into());
        self
    }

    /// 设置需求分析（可选）
    pub fn requirement_analysis(mut self, analysis: impl Into<String>) -> Self {
        self.requirement_analysis = Some(analysis.into());
        self
    }

    /// 设置解决思路及方法（必须）
    pub fn solution_approach(mut self, approach: impl Into<String>) -> Self {
        self.solution_approach = Some(approach.into());
        self
    }

    /// 添加遇到的错误（可选）
    pub fn add_error(mut self, error: impl Into<String>) -> Self {
        self.errors_encountered
            .get_or_insert_with(Vec::new)
            .push(error.into());
        self
    }

    /// 设置解决结果（必须）
    pub fn solution_result(mut self, result: impl Into<String>) -> Self {
        self.solution_result = Some(result.into());
        self
    }

    /// 设置总结分析（可选）
    pub fn summary_analysis(mut self, summary: impl Into<String>) -> Self {
        self.summary_analysis = Some(summary.into());
        self
    }

    /// 设置实施方案（可选，短方案）
    pub fn implementation_plan(mut self, plan: impl Into<String>) -> Self {
        self.implementation_plan = Some(plan.into());
        self
    }

    /// 设置方案文件位置（可选，长方案）
    pub fn plan_file_path(mut self, path: impl Into<String>) -> Self {
        self.plan_file_path = Some(path.into());
        self
    }

    /// 设置评价（必须）
    pub fn evaluation(mut self, eval: impl Into<String>) -> Self {
        self.evaluation = Some(eval.into());
        self
    }

    /// 添加关键决策（可选）
    pub fn add_key_decision(mut self, decision: impl Into<String>) -> Self {
        self.key_decisions
            .get_or_insert_with(Vec::new)
            .push(decision.into());
        self
    }

    /// 添加技术选型（可选）
    pub fn add_tech_stack(mut self, tech: impl Into<String>) -> Self {
        self.tech_stack
            .get_or_insert_with(Vec::new)
            .push(tech.into());
        self
    }

    /// 构建最终的markdown格式内容
    ///
    /// 返回结构化的markdown文本，包含所有必须和可选字段
    pub fn build(self) -> anyhow::Result<String> {
        let mut content = String::new();

        // 必须项检查
        let user_req = self
            .user_requirement
            .ok_or_else(|| anyhow::anyhow!("用户需求是必须的"))?;
        let solution = self
            .solution_approach
            .ok_or_else(|| anyhow::anyhow!("解决思路是必须的"))?;
        let result = self
            .solution_result
            .ok_or_else(|| anyhow::anyhow!("解决结果是必须的"))?;
        let evaluation = self
            .evaluation
            .ok_or_else(|| anyhow::anyhow!("评价是必须的"))?;

        // 1. 用户需求（必须）
        writeln!(content, "**用户需求**: {}", user_req)?;
        writeln!(content)?;

        // 2. 需求分析（可选）
        if let Some(analysis) = self.requirement_analysis {
            writeln!(content, "**需求分析**:")?;
            writeln!(content, "{}", analysis)?;
            writeln!(content)?;
        }

        // 3. 解决思路及方法（必须）
        writeln!(content, "**解决思路及方法**:")?;
        writeln!(content, "{}", solution)?;
        writeln!(content)?;

        // 4. 遇到的错误（可选）
        if let Some(errors) = self.errors_encountered {
            if !errors.is_empty() {
                writeln!(content, "**遇到的错误**:")?;
                for error in errors {
                    writeln!(content, "- {}", error)?;
                }
                writeln!(content)?;
            }
        }

        // 5. 解决结果（必须）
        writeln!(content, "**解决结果**: {}", result)?;
        writeln!(content)?;

        // 6. 总结分析（可选）
        if let Some(summary) = self.summary_analysis {
            writeln!(content, "**总结分析**:")?;
            writeln!(content, "{}", summary)?;
            writeln!(content)?;
        }

        // 7. 实施方案（可选）
        if let Some(plan) = self.implementation_plan {
            writeln!(content, "**实施方案**:")?;
            writeln!(content, "{}", plan)?;
            writeln!(content)?;
        }

        // 8. 方案文件位置（如果有）
        if let Some(path) = self.plan_file_path {
            writeln!(content, "**方案文件**: `{}`", path)?;
            writeln!(content)?;
        }

        // 9. 评价（必须）
        writeln!(content, "**评价**: {}", evaluation)?;
        writeln!(content)?;

        // 10. 关键决策（可选）
        if let Some(decisions) = self.key_decisions {
            if !decisions.is_empty() {
                writeln!(content, "**关键决策**:")?;
                for decision in decisions {
                    writeln!(content, "- {}", decision)?;
                }
                writeln!(content)?;
            }
        }

        // 11. 技术选型（可选）
        if let Some(tech_stack) = self.tech_stack {
            if !tech_stack.is_empty() {
                writeln!(content, "**技术选型**:")?;
                for tech in tech_stack {
                    writeln!(content, "- {}", tech)?;
                }
                writeln!(content)?;
            }
        }

        Ok(content)
    }

    /// 从对话上下文提取并构建bullet
    ///
    /// 这个方法分析对话内容，尝试提取必须的字段
    pub fn from_conversation(
        user_query: &str,
        conversation: &str,
        success: bool,
    ) -> anyhow::Result<String> {
        let mut builder = Self::new();

        // 1. 用户需求 = user_query
        builder = builder.user_requirement(user_query);

        // 2. 从conversation中提取解决思路
        // 简化版：取conversation的前部分作为思路
        let approach = Self::extract_approach(conversation);
        builder = builder.solution_approach(approach);

        // 3. 解决结果
        let result = if success {
            "任务成功完成"
        } else {
            "任务执行失败或部分完成"
        };
        builder = builder.solution_result(result);

        // 4. 评价
        let evaluation = if success {
            "✅ 成功"
        } else {
            "⚠️  需要改进"
        };
        builder = builder.evaluation(evaluation);

        // 5. 如果失败，尝试提取错误
        if !success {
            if let Some(error) = Self::extract_error(conversation) {
                builder = builder.add_error(error);
            }
        }

        builder.build()
    }

    /// 提取解决思路（简化版）
    fn extract_approach(conversation: &str) -> String {
        // 取对话的摘要（前500字符）
        let summary: String = conversation.chars().take(500).collect();
        if summary.len() < conversation.len() {
            format!("{}...", summary.trim())
        } else {
            summary.trim().to_string()
        }
    }

    /// 提取错误信息（简化版）
    fn extract_error(conversation: &str) -> Option<String> {
        let lower = conversation.to_lowercase();
        if lower.contains("error") || lower.contains("failed") || lower.contains("错误") {
            // 简化：返回包含error的第一行
            for line in conversation.lines() {
                let line_lower = line.to_lowercase();
                if line_lower.contains("error")
                    || line_lower.contains("failed")
                    || line_lower.contains("错误")
                {
                    return Some(line.trim().to_string());
                }
            }
        }
        None
    }
}

impl Default for BulletContentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_complete_bullet() {
        let content = BulletContentBuilder::new()
            .user_requirement("实现用户登录功能")
            .requirement_analysis("需要支持邮箱和密码登录，包含验证和会话管理")
            .solution_approach("使用JWT token进行认证，bcrypt加密密码")
            .add_error("初次尝试时忘记添加密码盐值")
            .solution_result("成功实现登录功能，包含完整的安全机制")
            .summary_analysis("JWT方案简单高效，适合无状态API")
            .evaluation("✅ 成功，安全性良好")
            .add_key_decision("选择JWT而非session")
            .add_tech_stack("jsonwebtoken crate")
            .build()
            .unwrap();

        assert!(content.contains("用户需求"));
        assert!(content.contains("JWT token"));
        assert!(content.contains("成功"));
    }

    #[test]
    fn test_build_minimal_bullet() {
        let content = BulletContentBuilder::new()
            .user_requirement("测试命令")
            .solution_approach("使用cargo test")
            .solution_result("测试通过")
            .evaluation("✅ 成功")
            .build()
            .unwrap();

        assert!(content.contains("用户需求"));
        assert!(content.contains("cargo test"));
    }

    #[test]
    fn test_missing_required_field() {
        let result = BulletContentBuilder::new()
            .user_requirement("测试")
            .solution_approach("方法")
            // 缺少solution_result
            .evaluation("好")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_from_conversation() {
        let content = BulletContentBuilder::from_conversation(
            "如何运行测试？",
            "可以使用cargo test命令运行所有测试。这会编译并执行测试用例。",
            true,
        )
        .unwrap();

        assert!(content.contains("如何运行测试"));
        assert!(content.contains("成功"));
    }
}
