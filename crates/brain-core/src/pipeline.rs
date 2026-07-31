//! Обработочный пайплайн — АЛГОРИТМИЧЕСКИЙ ДВИЖОК.
//!
//! Последовательно пропускает текст через все этапы обработки.
//! AI вызывается ТОЛЬКО на шагах классификации, и только если
//! алгоритмические правила недостаточно уверены.

use crate::traits::*;
use brain_common::{
    BrainEntry, Classification, EntryId, EntrySource, Result,
};
use std::sync::Arc;

/// Пайплайн обработки: текст → BrainEntry.
pub struct Pipeline {
    type_classifier: Arc<dyn TypeClassifier>,
    area_detector: Arc<dyn AreaDetector>,
    entity_extractor: Arc<dyn EntityExtractor>,
    tag_generator: Arc<dyn TagGenerator>,
    para_router: Arc<dyn ParaRouter>,
    title_generator: Arc<dyn TitleGenerator>,
    link_suggester: Arc<dyn LinkSuggester>,
}

impl Pipeline {
    /// Обработать текст пользователя через полный пайплайн:
    ///
    /// 1. Классифицировать тип (правила → AI fallback)
    /// 2. Определить область (ключевые слова → AI)
    /// 3. Извлечь сущности (regex + AI)
    /// 4. Сгенерировать теги (детерминированно)
    /// 5. Маршрутизировать в PARA (чистый алгоритм)
    /// 6. Сгенерировать заголовок (шаблон + AI)
    /// 7. Предложить связи (cosine similarity)
    pub async fn process(&self, raw_text: &str, source: EntrySource, context_str: &str) -> Result<BrainEntry> {
        tracing::info!("Pipeline: processing new entry");

        // Шаг 1: Классификация типа
        let (entry_type, type_confidence) = self.type_classifier.classify_type(raw_text, context_str).await?;
        tracing::debug!("Type: {} (confidence: {:.2})", entry_type, type_confidence);

        // Шаг 2: Определение области
        let (area, _area_confidence) = self.area_detector.detect_area(raw_text, &entry_type).await?;
        tracing::debug!("Area: {}", area);

        // Шаг 3: Извлечение сущностей
        let entities = self.entity_extractor.extract_entities(raw_text).await?;
        tracing::debug!("Entities: {}", entities.len());

        // Шаг 5: Маршрутизация PARA (чистый алгоритм, до тегов)
        let para_category = self.para_router.route(&entry_type, &area, raw_text).await?;
        tracing::debug!("PARA: {}", para_category);

        // Шаг 6: Генерация заголовка
        let suggested_title = self.title_generator.generate_title(raw_text, &entry_type, context_str).await?;

        // Шаг 7: Предложение связей
        let suggested_links = self.link_suggester.suggest_links(raw_text, 5, context_str).await?;

        // Собираем промежуточную классификацию для генерации тегов
        let mut classification = Classification {
            entry_type,
            area,
            para_category,
            entities,
            tags: vec![],
            confidence: type_confidence,
            suggested_title,
            suggested_links,
            summary: String::new(),
        };

        // Шаг 4: Генерация тегов (после остальной классификации)
        let tags = self.tag_generator.generate_tags(raw_text, &classification, context_str).await?;
        classification.tags = tags;

        // Собираем BrainEntry
        let entry = BrainEntry {
            id: EntryId::new(),
            raw_text: raw_text.to_string(),
            classification,
            created_at: chrono::Utc::now(),
            source,
        };

        tracing::info!("Pipeline: entry processed — {}", entry.classification.suggested_title);
        Ok(entry)
    }
}

/// Builder для конструирования Pipeline.
pub struct PipelineBuilder {
    type_classifier: Option<Arc<dyn TypeClassifier>>,
    area_detector: Option<Arc<dyn AreaDetector>>,
    entity_extractor: Option<Arc<dyn EntityExtractor>>,
    tag_generator: Option<Arc<dyn TagGenerator>>,
    para_router: Option<Arc<dyn ParaRouter>>,
    title_generator: Option<Arc<dyn TitleGenerator>>,
    link_suggester: Option<Arc<dyn LinkSuggester>>,
}

impl PipelineBuilder {
    pub fn new() -> Self {
        Self {
            type_classifier: None,
            area_detector: None,
            entity_extractor: None,
            tag_generator: None,
            para_router: None,
            title_generator: None,
            link_suggester: None,
        }
    }

    pub fn type_classifier(mut self, c: Arc<dyn TypeClassifier>) -> Self { self.type_classifier = Some(c); self }
    pub fn area_detector(mut self, d: Arc<dyn AreaDetector>) -> Self { self.area_detector = Some(d); self }
    pub fn entity_extractor(mut self, e: Arc<dyn EntityExtractor>) -> Self { self.entity_extractor = Some(e); self }
    pub fn tag_generator(mut self, g: Arc<dyn TagGenerator>) -> Self { self.tag_generator = Some(g); self }
    pub fn para_router(mut self, r: Arc<dyn ParaRouter>) -> Self { self.para_router = Some(r); self }
    pub fn title_generator(mut self, g: Arc<dyn TitleGenerator>) -> Self { self.title_generator = Some(g); self }
    pub fn link_suggester(mut self, s: Arc<dyn LinkSuggester>) -> Self { self.link_suggester = Some(s); self }

    /// Собрать Pipeline. Все компоненты обязательны.
    pub fn build(self) -> Result<Pipeline> {
        use brain_common::BrainError;
        Ok(Pipeline {
            type_classifier: self.type_classifier.ok_or_else(|| BrainError::Config("TypeClassifier is required".into()))?,
            area_detector: self.area_detector.ok_or_else(|| BrainError::Config("AreaDetector is required".into()))?,
            entity_extractor: self.entity_extractor.ok_or_else(|| BrainError::Config("EntityExtractor is required".into()))?,
            tag_generator: self.tag_generator.ok_or_else(|| BrainError::Config("TagGenerator is required".into()))?,
            para_router: self.para_router.ok_or_else(|| BrainError::Config("ParaRouter is required".into()))?,
            title_generator: self.title_generator.ok_or_else(|| BrainError::Config("TitleGenerator is required".into()))?,
            link_suggester: self.link_suggester.ok_or_else(|| BrainError::Config("LinkSuggester is required".into()))?,
        })
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self { Self::new() }
}
