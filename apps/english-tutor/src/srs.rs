use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabCard {
    pub id: String,
    pub word: String,
    pub phonetic: String,
    pub translation_ru: String,
    pub example_en: String,
    pub example_ru: String,
    pub level: String, // A1, A2, B1, B2, C1, C2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSrsState {
    pub user_id: u64,
    pub card_id: String,
    pub repetitions: u32,
    pub interval_days: u32,
    pub ease_factor: f32,
    pub next_review: chrono::DateTime<chrono::Utc>,
}

impl UserSrsState {
    pub fn new(user_id: u64, card_id: impl Into<String>) -> Self {
        Self {
            user_id,
            card_id: card_id.into(),
            repetitions: 0,
            interval_days: 1,
            ease_factor: 2.5,
            next_review: chrono::Utc::now(),
        }
    }

    /// SuperMemo-2 (SM-2) algorithm update
    pub fn review(&mut self, quality: u8) {
        let q = quality.min(5) as f32;

        if q >= 3.0 {
            if self.repetitions == 0 {
                self.interval_days = 1;
            } else if self.repetitions == 1 {
                self.interval_days = 6;
            } else {
                self.interval_days = (self.interval_days as f32 * self.ease_factor).round() as u32;
            }
            self.repetitions += 1;
        } else {
            self.repetitions = 0;
            self.interval_days = 1;
        }

        self.ease_factor = (self.ease_factor + (0.1 - (5.0 - q) * (0.08 + (5.0 - q) * 0.02))).max(1.3);
        self.next_review = chrono::Utc::now() + chrono::Duration::days(self.interval_days as i64);
    }
}

pub fn default_vocab_bank() -> Vec<VocabCard> {
    vec![
        VocabCard {
            id: "v1".into(),
            word: "Resilient".into(),
            phonetic: "/rɪˈzɪl.jənt/".into(),
            translation_ru: "Устойчивый, жизнестойкий, гибкий".into(),
            example_en: "She is a resilient person who never gives up.".into(),
            example_ru: "Она жизнестойкий человек, который никогда не сдаётся.".into(),
            level: "B2".into(),
        },
        VocabCard {
            id: "v2".into(),
            word: "Ubiquitous".into(),
            phonetic: "/juːˈbɪk.wə.təs/".into(),
            translation_ru: "Вездесущий, повсеместный".into(),
            example_en: "Smartphones have become ubiquitous in modern society.".into(),
            example_ru: "Смартфоны стали повсеместными в современном обществе.".into(),
            level: "C1".into(),
        },
        VocabCard {
            id: "v3".into(),
            word: "Pragmatic".into(),
            phonetic: "/præɡˈmæt.ɪk/".into(),
            translation_ru: "Прагматичный, практичный".into(),
            example_en: "We need a pragmatic solution to this problem.".into(),
            example_ru: "Нам нужно практичное решение этой проблемы.".into(),
            level: "B2".into(),
        },
        VocabCard {
            id: "v4".into(),
            word: "Ambiguity".into(),
            phonetic: "/ˌæm.bɪˈɡjuː.ə.ti/".into(),
            translation_ru: "Двусмысленность, неопределенность".into(),
            example_en: "There is some ambiguity in the instructions.".into(),
            example_ru: "В инструкциях есть некоторая неопределенность.".into(),
            level: "C1".into(),
        },
        VocabCard {
            id: "v5".into(),
            word: "Consistent".into(),
            phonetic: "/kənˈsɪs.tənt/".into(),
            translation_ru: "Последовательный, постоянный".into(),
            example_en: "Consistent practice leads to mastery.".into(),
            example_ru: "Последовательная практика ведет к мастерству.".into(),
            level: "B1".into(),
        },
    ]
}
