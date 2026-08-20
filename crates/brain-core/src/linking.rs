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

fn find_safe_match(text: &str, alias: &str) -> Option<(usize, usize)> {
    let alias_chars: Vec<char> = alias.chars().flat_map(|c| c.to_lowercase()).collect();
    if alias_chars.is_empty() {
        return None;
    }

    let text_chars: Vec<(usize, char)> = text.char_indices().collect();
    if text_chars.is_empty() {
        return None;
    }

    for (start_idx, &(byte_start, _)) in text_chars.iter().enumerate() {
        if !is_safe_context(text, byte_start) {
            continue;
        }

        // Check start boundary: previous char must not be alphanumeric
        if start_idx > 0 && text_chars[start_idx - 1].1.is_alphanumeric() {
            continue;
        }

        let mut matched = true;
        let mut curr_idx = start_idx;

        for &ac in &alias_chars {
            if curr_idx >= text_chars.len() {
                matched = false;
                break;
            }
            let tc_low: Vec<char> = text_chars[curr_idx].1.to_lowercase().collect();
            if tc_low == vec![ac] {
                curr_idx += 1;
            } else {
                matched = false;
                break;
            }
        }

        if matched {
            // Check end boundary: next char (if any) must not be alphanumeric
            let is_end_boundary = if curr_idx < text_chars.len() {
                !text_chars[curr_idx].1.is_alphanumeric()
            } else {
                true
            };

            if is_end_boundary {
                let byte_end = if curr_idx < text_chars.len() {
                    text_chars[curr_idx].0
                } else {
                    text.len()
                };
                return Some((byte_start, byte_end));
            }
        }
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

    #[test]
    fn test_cyrillic_boundary_and_utf8_safety() {
        let docs = vec![
            LinkCandidate {
                id: "rust-1".into(),
                title: "Язык Rust".into(),
                aliases: vec!["раст".into(), "rust".into()],
                score: 0.95,
            },
            LinkCandidate {
                id: "ai-1".into(),
                title: "Искусственный интеллект".into(),
                aliases: vec!["ИИ".into()],
                score: 0.9,
            },
        ];

        // Ensure non-boundary matches like "растение" are not linked
        let text = "растение растет, но раст это язык программирования и ИИ в деле.";
        let res = auto_link(text, &docs);
        assert_eq!(res, "растение растет, но [[Язык Rust|раст]] это язык программирования и [[Искусственный интеллект|ИИ]] в деле.");
    }
}
