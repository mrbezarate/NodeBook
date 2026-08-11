use std::collections::HashMap;
use std::sync::Arc;
use crate::traits::{RawEventStore, VectorStorage, EmbeddingProvider};
use tracing::info;

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum Intent {
    Question,
    Note,
    Debug,
    Search,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct RetrievalWeights {
    pub vector: f32,
    pub graph: f32,
    pub recency: f32,
    pub mmr_lambda: f32,
}

pub fn detect_intent(text: &str) -> Intent {
    let t = text.to_lowercase();
    if t.contains("?") { return Intent::Question; }
    if t.contains("error") || t.contains("panic") { return Intent::Debug; }
    if t.starts_with("find ") || t.starts_with("search ") { return Intent::Search; }
    if text.len() > 200 { return Intent::Note; }
    Intent::Note
}

pub fn get_weights(intent: &Intent, text: &str) -> RetrievalWeights {
    if text.starts_with("!strict") { return RetrievalWeights { vector: 1.0, graph: 0.0, recency: 0.0, mmr_lambda: 1.0 }; }
    if text.starts_with("!explore") { return RetrievalWeights { vector: 0.0, graph: 1.0, recency: 0.0, mmr_lambda: 0.5 }; }

    match intent {
        Intent::Question => RetrievalWeights { vector: 0.8, graph: 0.2, recency: 0.3, mmr_lambda: 0.8 },
        Intent::Note => RetrievalWeights { vector: 0.4, graph: 0.6, recency: 0.7, mmr_lambda: 0.5 },
        Intent::Debug => RetrievalWeights { vector: 0.7, graph: 0.3, recency: 0.2, mmr_lambda: 0.7 },
        Intent::Search => RetrievalWeights { vector: 0.9, graph: 0.1, recency: 0.1, mmr_lambda: 0.9 },
    }
}

#[derive(Debug, serde::Serialize)]
pub struct RetrievalTrace {
    pub query: String,
    pub intent: Intent,
    pub weights: RetrievalWeights,
    pub vector_hits: Vec<(String, f32)>,
    pub graph_expansion: Vec<(String, f32, String)>, // (id, score, source_id)
    pub final_rank: Vec<(String, f32)>,
    pub mmr_penalties: Vec<f32>,
}

pub struct MmrCandidate {
    pub id: String,
    pub score: f32,
    pub entry: brain_common::ProjectionEntry,
    pub embedding: Vec<f32>,
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() { return 0.0; }
    let dot = a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm_a * norm_b + 1e-8)
}

fn mmr_select(mut candidates: Vec<MmrCandidate>, k: usize, lambda: f32) -> Vec<(MmrCandidate, f32)> {
    let mut selected = Vec::new();
    if candidates.is_empty() { return selected; }
    
    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    selected.push((candidates.remove(0), 0.0));
    
    while selected.len() < k && !candidates.is_empty() {
        let mut best_idx = 0;
        let mut best_score = f32::MIN;
        let mut best_sim = 0.0;
        
        for (i, candidate) in candidates.iter().enumerate() {
            let mut max_sim = 0.0_f32;
            for (sel, _) in &selected {
                let sim = cosine_similarity(&candidate.embedding, &sel.embedding);
                if sim > max_sim { max_sim = sim; }
            }
            let mmr = lambda * candidate.score - (1.0 - lambda) * max_sim;
            if mmr > best_score {
                best_score = mmr;
                best_idx = i;
                best_sim = max_sim;
            }
        }
        selected.push((candidates.remove(best_idx), best_sim));
    }
    selected
}

pub struct HybridRetriever {
    pub store: Arc<dyn RawEventStore>,
    pub vector_store: Option<Arc<dyn VectorStorage>>,
    pub embeddings: Option<Arc<dyn EmbeddingProvider>>,
}

impl HybridRetriever {
    pub fn new(
        store: Arc<dyn RawEventStore>, 
        vector_store: Option<Arc<dyn VectorStorage>>,
        embeddings: Option<Arc<dyn EmbeddingProvider>>,
    ) -> Self {
        Self { store, vector_store, embeddings }
    }

    pub async fn retrieve_context(&self, query: &str) -> (String, Vec<crate::linking::LinkCandidate>) {
        let intent = detect_intent(query);
        let weights = get_weights(&intent, query);

        let mut trace = RetrievalTrace {
            query: query.to_string(),
            intent,
            weights,
            vector_hits: vec![],
            graph_expansion: vec![],
            final_rank: vec![],
            mmr_penalties: vec![],
        };

        let mut scores: HashMap<String, f32> = HashMap::new();

        if let Some(ref embeddings) = self.embeddings {
            if let Some(ref vs) = self.vector_store {
                if let Ok(vector) = embeddings.embed(query).await {
                    if let Ok(semantic_matches) = vs.search(&vector, 10).await {
                        let mut vector_ids = vec![];
                        
                        for (id, score) in semantic_matches {
                            vector_ids.push(id.clone());
                            trace.vector_hits.push((id.clone(), score));
                            scores.insert(id, score * weights.vector);
                        }

                        // Batch fetch graph
                        if let Ok(graph_edges) = self.store.get_links_with_weights_batch(&vector_ids).await {
                            for (source_id, to_id, weight) in graph_edges {
                                let norm_score = 1.0 - (-weight / 5.0).exp();
                                
                                // Deduplication / Source weight decay
                                let final_score = if scores.contains_key(&to_id) {
                                    norm_score * weights.graph * 0.5 // Penalty if already found by vector
                                } else {
                                    norm_score * weights.graph
                                };

                                trace.graph_expansion.push((to_id.clone(), final_score, source_id));
                                
                                scores.entry(to_id)
                                    .and_modify(|s| *s += final_score)
                                    .or_insert(final_score);
                            }
                        }
                    }
                }
            }
        }

        let now = chrono::Utc::now();

        // 3. Apply Recency Decay and Minimum Threshold (0.15)
        let mut candidate_docs = vec![];
        for (id, mut score) in scores {
            if let Ok(Some(entry)) = self.store.load_projection(&id).await {
                // Decay
                let age_hours = (now - entry.created_at).num_hours() as f32;
                let recency = (-age_hours / 72.0).exp();
                score *= recency.powf(weights.recency);

                if score >= 0.15 {
                    let mut embedding = vec![];
                    if let Some(ref vs) = self.vector_store {
                        if let Ok(Some(vec)) = vs.get(&id).await {
                            embedding = vec;
                        }
                    }
                    candidate_docs.push(MmrCandidate { id, score, entry, embedding });
                }
            }
        }

        // 4. Apply MMR (Maximal Marginal Relevance) Selection
        let mmr_results = mmr_select(candidate_docs, 7, weights.mmr_lambda);
        
        let mut context_text = String::new();
        let mut candidates = vec![];

        for (cand, sim_penalty) in mmr_results {
            let id = cand.id;
            let score = cand.score;
            let entry = cand.entry;
            
            trace.final_rank.push((id.clone(), score));
            trace.mmr_penalties.push(sim_penalty);
            
            if !entry.summary.is_empty() {
                context_text.push_str(&format!("- [{}] {}\n", entry.tags.join(", "), entry.summary));
                
                let display_title = if entry.title.is_empty() { id.clone() } else { entry.title.clone() };
                candidates.push(crate::linking::LinkCandidate {
                    id: id.clone(),
                    title: display_title,
                    aliases: entry.tags.clone(),
                    score,
                });
            }
        }

        info!(target: "retrieval", trace = ?trace, "Hybrid Retrieval completed");

        (context_text, candidates)
    }
}
