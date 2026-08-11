use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct LinkCandidate {
    pub id: String,
    pub title: String,
    pub aliases: Vec<String>,
    pub score: f32,
}

pub fn auto_link(text: &str, docs: &[LinkCandidate]) -> String {
    let mut result = text.to_string();
    let mut linked_ids = HashSet::new();
    let mut link_count = 0;
    let max_links = 5;

    // Sort docs by score DESC
    let mut sorted_docs = docs.to_vec();
    sorted_docs.sort_by(|a, b| {
        b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
    });

    for doc in sorted_docs {
        if link_count >= max_links { break; }
        if doc.score < 0.3 { continue; }
        if linked_ids.contains(&doc.id) { continue; }

        let mut matched = false;
        
        let mut candidates = vec![doc.title.clone()];
        candidates.extend(doc.aliases.clone());
        
        // Sort aliases by length DESC to match longest first
        candidates.sort_by(|a, b| b.len().cmp(&a.len()));

        for alias in candidates {
            if alias.len() < 3 { continue; }
            if matched { break; }

            // 1. Try exact safe match first
            if let Some((start, end)) = find_safe_match(&result, &alias) {
                let matched_text = &result[start..end];
                let replacement = if matched_text.to_lowercase() == doc.title.to_lowercase() {
                    format!("[[{}]]", doc.title)
                } else {
                    format!("[[{}|{}]]", doc.title, matched_text)
                };

                result.replace_range(start..end, &replacement);
                matched = true;
                linked_ids.insert(doc.id.clone());
                link_count += 1;
                continue;
            }

            // 2. Try fuzzy safe match (for typos e.g. "кавбой бибоп" -> "ковбой бибоп")
            if let Some((start, end)) = find_safe_fuzzy_match(&result, &alias) {
                let matched_text = &result[start..end];
                let replacement = if matched_text.to_lowercase() == doc.title.to_lowercase() {
                    format!("[[{}]]", doc.title)
                } else {
                    format!("[[{}|{}]]", doc.title, matched_text)
                };

                result.replace_range(start..end, &replacement);
                matched = true;
                linked_ids.insert(doc.id.clone());
                link_count += 1;
            }
        }
    }

    result
}

/// Helper to find an exact substring that is NOT inside a code block, existing link, or URL
fn find_safe_match(text: &str, alias: &str) -> Option<(usize, usize)> {
    let lower_text = text.to_lowercase();
    let lower_alias = alias.to_lowercase();
    
    let mut start_idx = 0;
    while let Some(idx) = lower_text[start_idx..].find(&lower_alias) {
        let abs_idx = start_idx + idx;
        let end_idx = abs_idx + alias.len();

        if is_safe_context(text, abs_idx) {
            let is_start_boundary = abs_idx == 0 || !is_alphanumeric_or_cyrillic(text, abs_idx - 1);
            let is_end_boundary = end_idx == text.len() || !is_alphanumeric_or_cyrillic(text, end_idx);
            
            if is_start_boundary && is_end_boundary {
                return Some((abs_idx, end_idx));
            }
        }
        start_idx = abs_idx + 1;
    }
    None
}

/// Fuzzy matcher using sliding window of words and Levenshtein distance
fn find_safe_fuzzy_match(text: &str, alias: &str) -> Option<(usize, usize)> {
    let alias_words: Vec<&str> = alias.split_whitespace().collect();
    if alias_words.is_empty() { return None; }
    
    let target_word_count = alias_words.len();
    let max_allowed_distance = match alias.chars().count() {
        0..=4 => 1,
        5..=10 => 2,
        _ => 3,
    };

    // Extract word boundaries in text
    let mut word_spans = Vec::new();
    let mut in_word = false;
    let mut start = 0;

    for (i, c) in text.char_indices() {
        let is_char = c.is_alphanumeric();
        if is_char && !in_word {
            in_word = true;
            start = i;
        } else if !is_char && in_word {
            in_word = false;
            word_spans.push((start, i));
        }
    }
    if in_word {
        word_spans.push((start, text.len()));
    }

    if word_spans.len() < target_word_count { return None; }

    // Sliding window over word spans
    for i in 0..=(word_spans.len() - target_word_count) {
        let span_start = word_spans[i].0;
        let span_end = word_spans[i + target_word_count - 1].1;
        let window_text = &text[span_start..span_end];

        if is_safe_context(text, span_start) {
            let dist = levenshtein_distance(&window_text.to_lowercase(), &alias.to_lowercase());
            if dist <= max_allowed_distance {
                return Some((span_start, span_end));
            }
        }
    }

    None
}

fn is_alphanumeric_or_cyrillic(text: &str, byte_idx: usize) -> bool {
    if byte_idx >= text.len() { return false; }
    text[byte_idx..].chars().next().map(|c| c.is_alphanumeric()).unwrap_or(false)
}

fn is_safe_context(text: &str, idx: usize) -> bool {
    let before = &text[..idx];

    // Check code block ```
    let code_blocks_before = before.matches("```").count();
    if code_blocks_before % 2 != 0 { return false; }
    
    // Check inline code `
    let inline_code_before = before.matches('`').count();
    if inline_code_before % 2 != 0 { return false; }

    // Check existing links [[ ]]
    let open_links = before.matches("[[").count();
    let close_links = before.matches("]]").count();
    if open_links > close_links { return false; }
    
    // Check markdown links []()
    let open_md_links = before.matches('[').count();
    let close_md_links = before.matches(']').count();
    if open_md_links > close_md_links { return false; }
    
    // URL heuristic
    if before.ends_with('/') || before.ends_with("http") || before.ends_with("://") { return false; }

    true
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let len_a = a_chars.len();
    let len_b = b_chars.len();

    if len_a == 0 { return len_b; }
    if len_b == 0 { return len_a; }

    let mut cache: Vec<usize> = (0..=len_b).collect();

    for (i, ca) in a_chars.iter().enumerate() {
        let mut result = i + 1;
        let mut distance_b = i;
        for (j, cb) in b_chars.iter().enumerate() {
            let distance_a = distance_b;
            distance_b = cache[j + 1];
            result = if ca == cb {
                distance_a
            } else {
                1 + std::cmp::min(std::cmp::min(result, distance_b), distance_a)
            };
            cache[j + 1] = result;
        }
    }
    cache[len_b]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_auto_link() {
        let docs = vec![
            LinkCandidate {
                id: "1".into(),
                title: "tokio runtime".into(),
                aliases: vec!["tokio".into()],
                score: 0.9,
            },
        ];

        let text = "tokio падает с ошибкой.";
        let res = auto_link(text, &docs);
        assert_eq!(res, "[[tokio runtime|tokio]] падает с ошибкой.");
    }

    #[test]
    fn test_fuzzy_typo_auto_link() {
        let docs = vec![
            LinkCandidate {
                id: "bebop-1".into(),
                title: "впичетление об аниме кавбой бибоп".into(),
                aliases: vec!["ковбой бибоп".into()],
                score: 0.95,
            },
        ];

        let text = "я вчера смотрел кавбой бибоп";
        let res = auto_link(text, &docs);
        assert_eq!(res, "я вчера смотрел [[впичетление об аниме кавбой бибоп|кавбой бибоп]]");
    }
}
