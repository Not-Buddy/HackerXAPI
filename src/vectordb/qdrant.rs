use async_trait::async_trait;
use qdrant_client::config::QdrantConfig;
use qdrant_client::qdrant::{
    point_id, vector, vectors, vectors_output, Condition, CreateCollectionBuilder, DenseVector,
    Distance, Filter, PointId, PointStruct, RetrievedPoint, ScrollPointsBuilder,
    SearchPointsBuilder, UpsertPointsBuilder, Vector, VectorParamsBuilder, Vectors,
};
use qdrant_client::{Payload, Qdrant};

use crate::error::{AppError, Result};
use crate::vectordb::{ChunkEmbedding, ScoredChunk, VectorStore};

pub struct QdrantStore {
    client: Qdrant,
    collection_name: String,
}

impl QdrantStore {
    pub async fn new(
        url: &str,
        api_key: &str,
        collection_name: &str,
        vector_size: u64,
    ) -> Result<Self> {
        let mut config = QdrantConfig::from_url(url);
        config.check_compatibility = false;
        if !api_key.is_empty() {
            config.set_api_key(api_key);
        }
        let client = config
            .build()
            .map_err(|e| AppError::VectorStore(e.to_string()))?;

        let exists = client
            .collection_exists(collection_name)
            .await
            .map_err(|e| AppError::VectorStore(e.to_string()))?;

        if !exists {
            let create = CreateCollectionBuilder::new(collection_name)
                .vectors_config(VectorParamsBuilder::new(vector_size, Distance::Cosine))
                .build();

            client
                .create_collection(create)
                .await
                .map_err(|e| AppError::VectorStore(e.to_string()))?;
        }

        Ok(Self {
            client,
            collection_name: collection_name.to_string(),
        })
    }
}

fn doc_id_filter(doc_id: &str) -> Filter {
    Filter::must([Condition::matches("doc_id", doc_id.to_string())])
}

#[allow(deprecated)]
fn make_vector(embedding: &[f32]) -> Vector {
    Vector {
        data: vec![],
        indices: None,
        vectors_count: None,
        vector: Some(vector::Vector::Dense(DenseVector {
            data: embedding.to_vec(),
        })),
    }
}

fn embeddings_from_point(p: RetrievedPoint) -> Option<ChunkEmbedding> {
    let payload = &p.payload;
    let chunk_text = payload
        .get("chunk_text")
        .and_then(|v| v.kind.as_ref())
        .and_then(|k| {
            if let qdrant_client::qdrant::value::Kind::StringValue(s) = k {
                Some(s.clone())
            } else {
                None
            }
        })?;
    let chunk_index = payload
        .get("chunk_index")
        .and_then(|v| v.kind.as_ref())
        .and_then(|k| {
            if let qdrant_client::qdrant::value::Kind::IntegerValue(i) = k {
                Some(*i as u32)
            } else {
                None
            }
        })
        .unwrap_or(0);

    let vectors_output = p.vectors?;
    let vec_options = vectors_output.vectors_options?;
    let embedding = match vec_options {
        vectors_output::VectorsOptions::Vector(v) => {
            #[allow(deprecated)]
            {
                v.data
            }
        }
        _ => return None,
    };

    Some(ChunkEmbedding {
        chunk_text,
        chunk_index,
        embedding,
    })
}

fn scored_chunk_from_point(p: qdrant_client::qdrant::ScoredPoint) -> Option<ScoredChunk> {
    let payload = &p.payload;
    let chunk_text = payload
        .get("chunk_text")
        .and_then(|v| v.kind.as_ref())
        .and_then(|k| {
            if let qdrant_client::qdrant::value::Kind::StringValue(s) = k {
                Some(s.clone())
            } else {
                None
            }
        })?;
    let chunk_index = payload
        .get("chunk_index")
        .and_then(|v| v.kind.as_ref())
        .and_then(|k| {
            if let qdrant_client::qdrant::value::Kind::IntegerValue(i) = k {
                Some(*i as u32)
            } else {
                None
            }
        })
        .unwrap_or(0);
    Some(ScoredChunk {
        chunk_text,
        chunk_index,
        score: p.score,
    })
}

#[async_trait]
impl VectorStore for QdrantStore {
    async fn store_embeddings(&self, doc_id: &str, chunks: &[ChunkEmbedding]) -> Result<()> {
        let mut points = Vec::with_capacity(chunks.len());

        for c in chunks {
            let id = PointId {
                point_id_options: Some(point_id::PointIdOptions::Uuid(
                    uuid::Uuid::new_v4().to_string(),
                )),
            };

            let mut payload = Payload::new();
            payload.insert("doc_id", doc_id);
            payload.insert("chunk_text", c.chunk_text.clone());
            payload.insert("chunk_index", c.chunk_index as i64);

            points.push(PointStruct {
                id: Some(id),
                vectors: Some(Vectors {
                    vectors_options: Some(vectors::VectorsOptions::Vector(make_vector(
                        &c.embedding,
                    ))),
                }),
                payload: payload.into(),
            });
        }

        let upsert = UpsertPointsBuilder::new(&self.collection_name, points).build();

        self.client
            .upsert_points(upsert)
            .await
            .map_err(|e| AppError::VectorStore(e.to_string()))?;

        Ok(())
    }

    async fn get_embeddings(&self, doc_id: &str) -> Result<Vec<ChunkEmbedding>> {
        let scroll = ScrollPointsBuilder::new(&self.collection_name)
            .filter(doc_id_filter(doc_id))
            .limit(10_000)
            .with_payload(true)
            .with_vectors(true)
            .build();

        let response = self
            .client
            .scroll(scroll)
            .await
            .map_err(|e| AppError::VectorStore(e.to_string()))?;

        let mut embeddings: Vec<ChunkEmbedding> = response
            .result
            .into_iter()
            .filter_map(embeddings_from_point)
            .collect();

        embeddings.sort_by_key(|e| e.chunk_index);
        Ok(embeddings)
    }

    async fn embeddings_exist(&self, doc_id: &str) -> Result<bool> {
        let scroll = ScrollPointsBuilder::new(&self.collection_name)
            .filter(doc_id_filter(doc_id))
            .limit(1)
            .build();

        let response = self
            .client
            .scroll(scroll)
            .await
            .map_err(|e| AppError::VectorStore(e.to_string()))?;

        Ok(!response.result.is_empty())
    }

    async fn search_similar(
        &self,
        embedding: &[f32],
        top_k: usize,
        threshold: f32,
    ) -> Result<Vec<ScoredChunk>> {
        let search = SearchPointsBuilder::new(
            &self.collection_name,
            embedding.to_vec(),
            top_k as u64,
        )
        .score_threshold(threshold)
        .with_payload(true)
        .build();

        let response = self
            .client
            .search_points(search)
            .await
            .map_err(|e| AppError::VectorStore(e.to_string()))?;

        let results: Vec<ScoredChunk> = response
            .result
            .into_iter()
            .filter_map(scored_chunk_from_point)
            .collect();

        Ok(results)
    }
}
