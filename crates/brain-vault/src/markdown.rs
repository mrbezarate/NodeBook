//! Генерация Markdown для Obsidian.
use brain_common::BrainEntry;
use crate::frontmatter::Frontmatter;

pub struct MarkdownBuilder;

impl MarkdownBuilder {
    /// Сгенерировать полный markdown документ из BrainEntry.
    pub fn build(entry: &BrainEntry) -> String {
        let summary_opt = if entry.classification.summary.trim().is_empty() {
            None
        } else {
            Some(entry.classification.summary.clone())
        };

        let fm = Frontmatter {
            title: entry.classification.suggested_title.clone(),
            entry_type: format!("{:?}", entry.classification.entry_type),
            area: entry.classification.area.to_string(),
            para: format!("{:?}", entry.classification.para_category),
            tags: entry.classification.tags.clone(),
            created: entry.created_at.format("%Y-%m-%d %H:%M").to_string(),
            modified: entry.created_at.format("%Y-%m-%d %H:%M").to_string(),
            id: entry.id.to_string(),
            summary: summary_opt.clone(),
            links: entry.classification.suggested_links.clone(),
            source: format!("{:?}", entry.source),
        };
        let mut md = fm.to_yaml_block();
        md.push('\n');
        
        md.push_str(&format!("# {}\n\n", fm.title));

        if let Some(ref sum) = summary_opt {
            md.push_str(&format!("> 💡 **ИИ-выжимка:** {}\n\n---\n\n", sum));
        }

        let body = entry.classification.enriched_text.clone().unwrap_or_else(|| entry.raw_text.clone());
        md.push_str(&body);
        md.push_str("\n\n");

        if !entry.classification.tags.is_empty() {
            let tags_str = entry.classification.tags.iter().map(|t| format!("#{}", t)).collect::<Vec<_>>().join(" ");
            md.push_str(&tags_str);
            md.push('\n');
        }

        md
    }
}
