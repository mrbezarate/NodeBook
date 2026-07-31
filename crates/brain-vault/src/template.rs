//! Шаблоны заметок для разных типов записей.
pub struct TemplateEngine;
impl TemplateEngine {
    pub fn diary_template(date: &str) -> String { format!("# 📔 Дневник — {date}\n\n## Оценки\n\n## Хорошее\n\n## Плохое\n\n## Мысли\n") }
    pub fn idea_template(title: &str) -> String { format!("# 💡 {title}\n\n## Описание\n\n## Следующие шаги\n\n## Связи\n") }
    pub fn project_template(title: &str) -> String { format!("# 🚀 {title}\n\n## Цель\n\n## Задачи\n- [ ] \n\n## Заметки\n\n## Ресурсы\n") }
    pub fn knowledge_template(title: &str) -> String { format!("# 📚 {title}\n\n## Суть\n\n## Детали\n\n## Примеры\n\n## Источники\n") }
    pub fn person_template(name: &str) -> String { format!("# 👤 {name}\n\n## Контакты\n\n## Заметки\n\n## Встречи\n") }
}
