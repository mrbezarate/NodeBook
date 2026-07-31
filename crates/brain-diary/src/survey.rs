//! Вопросы обзора.
pub struct SurveyQuestion { pub key: &'static str, pub text_ru: &'static str, pub answer_type: AnswerType }
pub enum AnswerType { Scale1to10, YesNo, FreeText, Number }

pub fn all_questions() -> Vec<SurveyQuestion> {
    vec![
        SurveyQuestion { key: "day_rating", text_ru: "Оценка дня", answer_type: AnswerType::Scale1to10 },
        SurveyQuestion { key: "mood", text_ru: "Настроение", answer_type: AnswerType::Scale1to10 },
        SurveyQuestion { key: "energy", text_ru: "Энергия", answer_type: AnswerType::Scale1to10 },
        SurveyQuestion { key: "stress", text_ru: "Стресс", answer_type: AnswerType::Scale1to10 },
        SurveyQuestion { key: "motivation", text_ru: "Мотивация", answer_type: AnswerType::Scale1to10 },
        SurveyQuestion { key: "productivity", text_ru: "Продуктивность", answer_type: AnswerType::Scale1to10 },
        SurveyQuestion { key: "sleep", text_ru: "Часов сна", answer_type: AnswerType::Number },
        SurveyQuestion { key: "exercise", text_ru: "Тренировка", answer_type: AnswerType::YesNo },
        SurveyQuestion { key: "good", text_ru: "Хорошие события", answer_type: AnswerType::FreeText },
        SurveyQuestion { key: "bad", text_ru: "Плохие события", answer_type: AnswerType::FreeText },
        SurveyQuestion { key: "thoughts", text_ru: "Свободные мысли", answer_type: AnswerType::FreeText },
    ]
}
