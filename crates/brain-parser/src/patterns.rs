//! Паттерны для классификации текста. Чистый алгоритм — regex + ключевые слова.

use regex::Regex;
use std::sync::LazyLock;

/// Совпадение паттерна.
#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub label: String,
    pub confidence: f32,
    pub matched_text: String,
}

/// Один компилированный паттерн.
struct CompiledPattern {
    regex: Regex,
    confidence: f32,
    label: String,
}

/// Набор паттернов для одной категории.
struct PatternSet {
    patterns: Vec<CompiledPattern>,
}

impl PatternSet {
    fn new(items: &[(&str, f32, &str)]) -> Self {
        Self {
            patterns: items.iter().map(|(pattern, conf, label)| {
                CompiledPattern {
                    regex: Regex::new(&format!("(?i){}", pattern)).unwrap(),
                    confidence: *conf,
                    label: label.to_string(),
                }
            }).collect(),
        }
    }

    fn find_matches(&self, text: &str) -> Vec<PatternMatch> {
        self.patterns.iter().filter_map(|p| {
            p.regex.find(text).map(|m| PatternMatch {
                label: p.label.clone(),
                confidence: p.confidence,
                matched_text: m.as_str().to_string(),
            })
        }).collect()
    }
}

// ── Компилированные наборы паттернов ────────────────────────

static TASK_PATTERNS: LazyLock<PatternSet> = LazyLock::new(|| PatternSet::new(&[
    (r"\b(?:нужно|надо)\b", 0.85, "task_need"),
    (r"\b(?:сделать|сделай)\b", 0.80, "task_do"),
    (r"\b(?:купить|купи)\b", 0.85, "task_buy"),
    (r"\bзадача\b", 0.90, "task_direct"),
    (r"\btodo\b", 0.90, "task_todo"),
    (r"\b(?:не забыть|напомни)\b", 0.80, "task_remind"),
    (r"\b(?:запланировать|запланируй)\b", 0.85, "task_plan"),
]));

static IDEA_PATTERNS: LazyLock<PatternSet> = LazyLock::new(|| PatternSet::new(&[
    (r"\bидея\b", 0.90, "idea_direct"),
    (r"\b(?:придумал|придумала)\b", 0.85, "idea_came_up"),
    (r"\bа что если\b", 0.80, "idea_what_if"),
    (r"\bможно было бы\b", 0.75, "idea_could"),
    (r"\bбыло бы круто\b", 0.75, "idea_cool"),
]));

static GOAL_PATTERNS: LazyLock<PatternSet> = LazyLock::new(|| PatternSet::new(&[
    (r"\bцель\b", 0.90, "goal_direct"),
    (r"\bхочу достичь\b", 0.85, "goal_achieve"),
    (r"\bплан на\b", 0.75, "goal_plan"),
    (r"\bстремлюсь\b", 0.75, "goal_aspire"),
]));

static DIARY_PATTERNS: LazyLock<PatternSet> = LazyLock::new(|| PatternSet::new(&[
    (r"\bсегодня\b.*\b(?:был|была|было)\b", 0.80, "diary_today"),
    (r"\b(?:чувствую|чувствовал)\b", 0.85, "diary_feel"),
    (r"\bнастроение\b", 0.85, "diary_mood"),
    (r"\bдень был\b", 0.80, "diary_day_was"),
    (r"\bустал|устала\b", 0.75, "diary_tired"),
]));

static FINANCE_PATTERNS: LazyLock<PatternSet> = LazyLock::new(|| PatternSet::new(&[
    (r"\b(?:купил|купила)\b.*\d", 0.85, "finance_bought"),
    (r"\b(?:потратил|потратила)\b", 0.85, "finance_spent"),
    (r"\b(?:заработал|заработала)\b", 0.85, "finance_earned"),
    (r"\d+\s*(?:руб|₽|р\.)", 0.80, "finance_rub"),
    (r"\$\s*\d+", 0.80, "finance_usd"),
]));

static KNOWLEDGE_PATTERNS: LazyLock<PatternSet> = LazyLock::new(|| PatternSet::new(&[
    (r"\b(?:понял|поняла)\b", 0.80, "knowledge_understood"),
    (r"\b(?:узнал|узнала)\b", 0.80, "knowledge_learned"),
    (r"\b(?:выучил|выучила)\b", 0.85, "knowledge_studied"),
    (r"\b(?:разобрался|разобралась)\b", 0.85, "knowledge_figured"),
    (r"\b(?:оказывается|выяснилось)\b", 0.75, "knowledge_turns_out"),
]));

static HABIT_PATTERNS: LazyLock<PatternSet> = LazyLock::new(|| PatternSet::new(&[
    (r"\bпривычка\b", 0.90, "habit_direct"),
    (r"\bкаждый день\b", 0.75, "habit_daily"),
    (r"\bstreak\b", 0.80, "habit_streak"),
    (r"\bрегулярно\b", 0.75, "habit_regular"),
]));

static PERSON_PATTERNS: LazyLock<PatternSet> = LazyLock::new(|| PatternSet::new(&[
    (r"\b(?:встретил|встретила)\b", 0.80, "person_met"),
    (r"\b(?:поговорил|поговорила)\s+с\b", 0.80, "person_talked"),
    (r"\b(?:познакомился|познакомилась)\b", 0.85, "person_met_new"),
]));

static BOOK_PATTERNS: LazyLock<PatternSet> = LazyLock::new(|| PatternSet::new(&[
    (r"\b(?:прочитал|прочитала)\b", 0.80, "book_read"),
    (r"\bкнига\b", 0.85, "book_direct"),
    (r"\bглава\b", 0.75, "book_chapter"),
    (r"\bстатья\b", 0.80, "article"),
]));

/// Движок сопоставления паттернов.
pub struct PatternMatcher;

impl PatternMatcher {
    /// Найти все совпадения паттернов и вернуть лучшее.
    /// Возвращает (категория, уверенность, все совпадения).
    pub fn match_all(text: &str) -> Vec<(&'static str, Vec<PatternMatch>)> {
        let categories: Vec<(&str, &PatternSet)> = vec![
            ("task", &*TASK_PATTERNS),
            ("idea", &*IDEA_PATTERNS),
            ("goal", &*GOAL_PATTERNS),
            ("diary", &*DIARY_PATTERNS),
            ("finance", &*FINANCE_PATTERNS),
            ("knowledge", &*KNOWLEDGE_PATTERNS),
            ("habit", &*HABIT_PATTERNS),
            ("person", &*PERSON_PATTERNS),
            ("book", &*BOOK_PATTERNS),
        ];

        categories.into_iter()
            .map(|(name, set)| (name, set.find_matches(text)))
            .filter(|(_, matches)| !matches.is_empty())
            .collect()
    }

    /// Найти лучшее совпадение — категорию с наивысшей уверенностью.
    pub fn best_match(text: &str) -> Option<(&'static str, f32)> {
        let all = Self::match_all(text);
        all.into_iter()
            .flat_map(|(name, matches)| {
                matches.into_iter().map(move |m| (name, m.confidence))
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    }
}
