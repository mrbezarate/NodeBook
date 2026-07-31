use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Relation { RelatedTo, PartOf, Influences, Contradicts, Extends, References, TaggedWith, BelongsToArea }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge { pub from: String, pub to: String, pub relation: Relation, pub weight: f32, pub created_at: DateTime<Utc> }
