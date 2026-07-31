//! Генерация Markdown для Obsidian.
use brain_common::BrainEntry;
use crate::frontmatter::Frontmatter;

pub struct MarkdownBuilder;

impl MarkdownBuilder {
    /// Сгенерировать полный markdown документ из BrainEntry.
    pub fn build(entry: &BrainEntry) -> String {
        let fm = Frontmatter {
            title: entry.classification.suggested_title.clone(),
            entry_type: format!("{:?}", entry.classification.entry_type),
            area: entry.classification.area.to_string(),
            para: format!("{:?}", entry.classification.para_category),
            tags: entry.classification.tags.clone(),
            created: entry.created_at.format("%Y-%m-%d %H:%M").to_string(),
            modified: entry.created_at.format("%Y-%m-%d %H:%M").to_string(),
            id: entry.id.to_string(),
            links: entry.classification.suggested_links.clone(),
            source: format!("{:?}", entry.source),
        };
        let mut md = fm.to_yaml_block();
        md.push('\n');
        
        md.push_str(&format!("# {}\n\n", fm.title));

        if !entry.classification.summary.is_empty() {
            md.push_str(&entry.classification.summary);
            md.push_str("\n\n");
        }

        md.push_str("## Оригинальная запись\n\n");
        md.push_str(&entry.raw_text);
        md.push_str("\n\n");

        if !entry.classification.suggested_links.is_empty() || !entry.classification.entities.is_empty() {
            md.push_str("## Граф Знаний\n\n");
            
            for link in &entry.classification.suggested_links {
                md.push_str(&format!("- **{}**: [[{}]]\n", link.relation, link.target));
            }
            for e in &entry.classification.entities {
                md.push_str(&format!("- Упоминается: [[{}]] ({:?})\n", e.name, e.entity_type));
            }
            md.push('\n');
        }

        if !entry.classification.tags.is_empty() {
            let tags_str = entry.classification.tags.iter().map(|t| format!("#{}", t)).collect::<Vec<_>>().join(" ");
            md.push_str(&tags_str);
            md.push('\n');
        }

        md
    }
}
