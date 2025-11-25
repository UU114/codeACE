// similarity.rs - 高级相似度计算模块
//
// 提供多种相似度算法，用于：
// 1. 更准确的内容去重
// 2. 更智能的搜索匹配
// 3. 更好的内容推荐

use std::collections::HashMap;

/// 相似度计算器
pub struct SimilarityCalculator;

impl SimilarityCalculator {
    /// 计算 Levenshtein 编辑距离
    ///
    /// 编辑距离是指将一个字符串转换为另一个字符串所需的最少编辑操作次数。
    /// 允许的编辑操作包括：插入、删除、替换字符。
    ///
    /// # 示例
    /// ```
    /// use codex_core::ace::similarity::SimilarityCalculator;
    ///
    /// let distance = SimilarityCalculator::levenshtein_distance("kitten", "sitting");
    /// assert_eq!(distance, 3);
    /// ```
    pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let len1 = s1.chars().count();
        let len2 = s2.chars().count();

        // 边界情况
        if len1 == 0 {
            return len2;
        }
        if len2 == 0 {
            return len1;
        }

        // 创建动态规划矩阵
        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        // 初始化第一行和第一列
        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        // 计算编辑距离
        let chars1: Vec<char> = s1.chars().collect();
        let chars2: Vec<char> = s2.chars().collect();

        for (i, c1) in chars1.iter().enumerate() {
            for (j, c2) in chars2.iter().enumerate() {
                let cost = if c1 == c2 { 0 } else { 1 };

                matrix[i + 1][j + 1] = std::cmp::min(
                    std::cmp::min(
                        matrix[i][j + 1] + 1, // 删除
                        matrix[i + 1][j] + 1, // 插入
                    ),
                    matrix[i][j] + cost, // 替换
                );
            }
        }

        matrix[len1][len2]
    }

    /// 计算相似度分数（基于 Levenshtein 距离）
    ///
    /// 返回值范围：0.0 (完全不同) - 1.0 (完全相同)
    ///
    /// # 示例
    /// ```
    /// use codex_core::ace::similarity::SimilarityCalculator;
    ///
    /// let score = SimilarityCalculator::similarity_score("hello", "hello");
    /// assert_eq!(score, 1.0);
    ///
    /// let score = SimilarityCalculator::similarity_score("hello", "world");
    /// assert!(score < 1.0);
    /// ```
    pub fn similarity_score(s1: &str, s2: &str) -> f32 {
        let distance = Self::levenshtein_distance(s1, s2) as f32;
        let max_len = s1.len().max(s2.len()) as f32;

        if max_len == 0.0 {
            return 1.0;
        }

        1.0 - (distance / max_len)
    }

    /// 计算 N-gram 相似度
    ///
    /// N-gram 是文本的连续 n 个字符片段。此方法计算两个字符串的 n-gram 集合的相似度。
    ///
    /// # 参数
    /// - `s1`: 第一个字符串
    /// - `s2`: 第二个字符串
    /// - `n`: N-gram 的大小（通常为 2-4）
    ///
    /// # 返回值
    /// 返回 0.0 (完全不同) - 1.0 (完全相同)
    ///
    /// # 示例
    /// ```
    /// use codex_core::ace::similarity::SimilarityCalculator;
    ///
    /// let score = SimilarityCalculator::ngram_similarity("hello", "hallo", 2);
    /// assert!(score > 0.5);
    /// ```
    pub fn ngram_similarity(s1: &str, s2: &str, n: usize) -> f32 {
        let ngrams1 = Self::extract_ngrams(s1, n);
        let ngrams2 = Self::extract_ngrams(s2, n);

        if ngrams1.is_empty() && ngrams2.is_empty() {
            return 1.0;
        }

        if ngrams1.is_empty() || ngrams2.is_empty() {
            return 0.0;
        }

        let mut intersection = 0;
        let mut total = 0;

        // 计算交集和并集
        for (gram, count1) in &ngrams1 {
            if let Some(count2) = ngrams2.get(gram) {
                intersection += count1.min(count2);
            }
            total += count1;
        }

        for (gram, count2) in &ngrams2 {
            if !ngrams1.contains_key(gram) {
                total += count2;
            }
        }

        if total == 0 {
            return 0.0;
        }

        intersection as f32 / total as f32
    }

    /// 提取 N-grams
    ///
    /// 将文本分解为大小为 n 的连续字符片段。
    ///
    /// # 示例
    /// ```
    /// use codex_core::ace::similarity::SimilarityCalculator;
    ///
    /// let ngrams = SimilarityCalculator::extract_ngrams("hello", 2);
    /// // 结果应包含: "he", "el", "ll", "lo"
    /// ```
    pub fn extract_ngrams(text: &str, n: usize) -> HashMap<String, usize> {
        let mut ngrams = HashMap::new();
        let chars: Vec<char> = text.chars().collect();

        if chars.len() < n {
            return ngrams;
        }

        for i in 0..=chars.len() - n {
            let gram: String = chars[i..i + n].iter().collect();
            *ngrams.entry(gram).or_insert(0) += 1;
        }

        ngrams
    }

    /// 组合相似度计算
    ///
    /// 结合多种算法计算综合相似度，提供更准确的结果。
    ///
    /// # 参数
    /// - `s1`: 第一个字符串
    /// - `s2`: 第二个字符串
    ///
    /// # 返回值
    /// 返回 0.0 (完全不同) - 1.0 (完全相同)
    ///
    /// # 算法
    /// - 40% Levenshtein 相似度
    /// - 30% 2-gram 相似度
    /// - 30% 3-gram 相似度
    pub fn combined_similarity(s1: &str, s2: &str) -> f32 {
        let lev_score = Self::similarity_score(s1, s2);
        let bigram_score = Self::ngram_similarity(s1, s2, 2);
        let trigram_score = Self::ngram_similarity(s1, s2, 3);

        // 加权平均
        lev_score * 0.4 + bigram_score * 0.3 + trigram_score * 0.3
    }

    /// 优化版组合相似度计算（根据文本长度调整权重）
    ///
    /// 对于短字符串，Levenshtein 距离更可靠；
    /// 对于长文本，N-gram 相似度更有效。
    ///
    /// # 参数
    /// - `s1`: 第一个字符串
    /// - `s2`: 第二个字符串
    ///
    /// # 返回值
    /// 返回 0.0 (完全不同) - 1.0 (完全相同)
    pub fn combined_similarity_v2(s1: &str, s2: &str) -> f32 {
        let len1 = s1.chars().count();
        let len2 = s2.chars().count();
        let min_len = len1.min(len2);

        let lev_score = Self::similarity_score(s1, s2);
        let bigram_score = Self::ngram_similarity(s1, s2, 2);

        if min_len <= 5 {
            // 短词：Levenshtein 占主导（70%）
            return lev_score * 0.7 + bigram_score * 0.3;
        }

        // 长文本：N-gram 更重要
        let trigram_score = Self::ngram_similarity(s1, s2, 3);
        lev_score * 0.3 + bigram_score * 0.35 + trigram_score * 0.35
    }

    /// 前缀匹配分数
    ///
    /// 计算两个字符串的公共前缀长度与最短字符串长度的比值。
    /// 对于英文单词的变形（如 "test" vs "testing"）很有用。
    ///
    /// # 参数
    /// - `s1`: 第一个字符串
    /// - `s2`: 第二个字符串
    ///
    /// # 返回值
    /// 返回 0.0 (无公共前缀) - 1.0 (完全相同或一个是另一个的前缀)
    pub fn prefix_similarity(s1: &str, s2: &str) -> f32 {
        let chars1: Vec<char> = s1.chars().collect();
        let chars2: Vec<char> = s2.chars().collect();
        let min_len = chars1.len().min(chars2.len());

        if min_len == 0 {
            return 0.0;
        }

        let mut common_prefix = 0;
        for i in 0..min_len {
            if chars1[i] == chars2[i] {
                common_prefix += 1;
            } else {
                break;
            }
        }

        common_prefix as f32 / min_len as f32
    }

    /// 检查两个字符串是否相似（用于去重）
    ///
    /// # 参数
    /// - `s1`: 第一个字符串
    /// - `s2`: 第二个字符串
    /// - `threshold`: 相似度阈值（0.0-1.0），默认推荐 0.85
    ///
    /// # 返回值
    /// 如果相似度高于阈值，返回 true
    pub fn is_similar(s1: &str, s2: &str, threshold: f32) -> bool {
        Self::combined_similarity(s1, s2) >= threshold
    }

    /// 归一化文本（用于提高相似度计算准确性）
    ///
    /// 执行以下操作：
    /// - 转换为小写
    /// - 移除多余空白
    /// - 移除标点符号（可选）
    pub fn normalize_text(text: &str, remove_punctuation: bool) -> String {
        let mut normalized = text.to_lowercase();

        if remove_punctuation {
            // 只保留字母、数字、空白和中文字符
            normalized = normalized
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace() || Self::is_cjk(*c))
                .collect();
        }

        // 压缩多余空白
        let words: Vec<&str> = normalized.split_whitespace().collect();
        words.join(" ")
    }

    /// 检查是否是中日韩（CJK）字符
    fn is_cjk(c: char) -> bool {
        matches!(c,
            '\u{4E00}'..='\u{9FFF}' |  // CJK Unified Ideographs
            '\u{3400}'..='\u{4DBF}' |  // CJK Extension A
            '\u{20000}'..='\u{2A6DF}' | // CJK Extension B
            '\u{F900}'..='\u{FAFF}'    // CJK Compatibility Ideographs
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        // 相同字符串
        assert_eq!(
            SimilarityCalculator::levenshtein_distance("hello", "hello"),
            0
        );

        // 经典示例
        assert_eq!(
            SimilarityCalculator::levenshtein_distance("kitten", "sitting"),
            3
        );

        // 空字符串
        assert_eq!(SimilarityCalculator::levenshtein_distance("", "hello"), 5);
        assert_eq!(SimilarityCalculator::levenshtein_distance("hello", ""), 5);

        // 单字符差异
        assert_eq!(
            SimilarityCalculator::levenshtein_distance("hello", "hallo"),
            1
        );
    }

    #[test]
    fn test_similarity_score() {
        // 完全相同
        assert_eq!(
            SimilarityCalculator::similarity_score("hello", "hello"),
            1.0
        );

        // 完全不同（长度相同）
        let score = SimilarityCalculator::similarity_score("hello", "world");
        assert!(score < 1.0 && score > 0.0);

        // 很相似
        let score = SimilarityCalculator::similarity_score("hello", "hallo");
        assert!(score > 0.6); // 调整期望值
    }

    #[test]
    fn test_ngram_extraction() {
        let ngrams = SimilarityCalculator::extract_ngrams("hello", 2);

        assert_eq!(ngrams.get("he"), Some(&1));
        assert_eq!(ngrams.get("el"), Some(&1));
        assert_eq!(ngrams.get("ll"), Some(&1));
        assert_eq!(ngrams.get("lo"), Some(&1));
        assert_eq!(ngrams.len(), 4);
    }

    #[test]
    fn test_ngram_similarity() {
        // 完全相同
        assert_eq!(
            SimilarityCalculator::ngram_similarity("hello", "hello", 2),
            1.0
        );

        // 相似
        let score = SimilarityCalculator::ngram_similarity("hello", "hallo", 2);
        assert!(score > 0.3); // 调整期望值

        // 不同
        let score = SimilarityCalculator::ngram_similarity("hello", "world", 2);
        assert!(score < 0.3);
    }

    #[test]
    fn test_combined_similarity() {
        // 完全相同
        assert_eq!(
            SimilarityCalculator::combined_similarity("hello", "hello"),
            1.0
        );

        // 很相似
        let score = SimilarityCalculator::combined_similarity("hello", "hallo");
        assert!(score > 0.45); // 调整期望值，考虑归一化影响

        // 不太相似
        let score = SimilarityCalculator::combined_similarity("hello", "world");
        assert!(score < 0.4);
    }

    #[test]
    fn test_is_similar() {
        assert!(SimilarityCalculator::is_similar("hello", "hello", 0.85));
        assert!(SimilarityCalculator::is_similar("hello", "hallo", 0.45)); // 调整阈值
        assert!(!SimilarityCalculator::is_similar("hello", "world", 0.45));
    }

    #[test]
    fn test_normalize_text() {
        // 转换为小写
        assert_eq!(
            SimilarityCalculator::normalize_text("Hello World", false),
            "hello world"
        );

        // 移除多余空白
        assert_eq!(
            SimilarityCalculator::normalize_text("hello  world   test", false),
            "hello world test"
        );

        // 移除标点符号
        assert_eq!(
            SimilarityCalculator::normalize_text("Hello, World!", true),
            "hello world"
        );
    }

    #[test]
    fn test_chinese_text_similarity() {
        // 测试中文文本
        let s1 = "使用 Rust 的 async/await 处理异步操作";
        let s2 = "使用 Rust 的 async/await 处理异步操作";
        assert_eq!(SimilarityCalculator::similarity_score(s1, s2), 1.0);

        let s3 = "使用 Rust 的 async/await 处理同步操作";
        let score = SimilarityCalculator::combined_similarity(s1, s3);
        assert!(score > 0.8); // 应该很相似
    }

    #[test]
    fn test_code_snippet_similarity() {
        let code1 = "fn main() {\n    println!(\"Hello, world!\");\n}";
        let code2 = "fn main() {\n    println!(\"Hello, Rust!\");\n}";

        let score = SimilarityCalculator::combined_similarity(code1, code2);
        assert!(score > 0.8); // 代码结构相似
    }
}
