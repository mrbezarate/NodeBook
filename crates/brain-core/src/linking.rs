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

    // Sort by score DESC, then by length DESC to match longer aliases first
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
            if alias.len() < 4 { continue; }
            if matched { break; }

            // Minimal protection against code blocks and existing links
            // We find the first occurrence that is NOT inside ` ` or [[ ]]
            if let Some((start, end)) = find_safe_match(&result, &alias) {
                let replacement = if alias.to_lowercase() == doc.title.to_lowercase() {
                    format!("[[{}]]", doc.title)
                } else {
                    format!("[[{}|{}]]", doc.title, alias)
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

// Helper to find a substring that is NOT inside a code block, existing link, or URL
fn find_safe_match(text: &str, alias: &str) -> Option<(usize, usize)> {
    let lower_text = text.to_lowercase();
    let lower_alias = alias.to_lowercase();
    
    let mut start_idx = 0;
    while let Some(idx) = lower_text[start_idx..].find(&lower_alias) {
        let abs_idx = start_idx + idx;
        let end_idx = abs_idx + alias.len();

        if is_safe_context(text, abs_idx) {
            // Also check word boundaries so we don't match partial words
            let is_start_boundary = abs_idx == 0 || !text.as_bytes()[abs_idx - 1].is_ascii_alphanumeric();
            let is_end_boundary = end_idx == text.len() || !text.as_bytes()[end_idx].is_ascii_alphanumeric();
            
            if is_start_boundary && is_end_boundary {
                return Some((abs_idx, end_idx));
            }
        }
        start_idx = abs_idx + 1;
    }
    None
}

fn is_safe_context(text: &str, idx: usize) -> bool {
    let before = &text[..idx];
    let after = &text[idx..];

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
    
    // URL heuristic (very basic)
    if before.ends_with("/") || before.ends_with("http") || before.ends_with("://") { return false; }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_link() {
        let docs = vec![
            LinkCandidate {
                id: "1".into(),
                title: "tokio runtime".into(),
                aliases: vec!["tokio".into()],
                score: 0.9,
            },
            LinkCandidate {
                id: "2".into(),
                title: "rust async".into(),
                aliases: vec![].into(),
                score: 0.2, // Should be skipped (< 0.3)
            }
        ];

        let text = "tokio падает с ошибкой, а rust async работает.";
        let res = auto_link(text, &docs);
        assert_eq!(res, "[[tokio runtime|tokio]] падает с ошибкой, а rust async работает.");
    }
}
