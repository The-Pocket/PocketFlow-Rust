use async_trait::async_trait;
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, DeletePointsBuilder, Distance, PointStruct, ScoredPoint,
    SearchPointsBuilder, UpsertPointsBuilder, VectorParamsBuilder,
};
use qdrant_client::qdrant::{Value as QdrantValue, value::Kind as QdrantKind};

use serde_json::{json, Map as SerdeMap, Number as SerdeNumber, Value as SerdeValue};

use tracing::info;

#[derive(Debug, Clone)]
pub struct VectorDBOptions {
    pub collection_name: String,
    pub dimension: usize,
    pub distance_metric: DistanceMetric,
}

#[derive(Debug, Clone)]
pub enum DistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
}

#[derive(Debug, Clone)]
pub struct VectorRecord {
    pub id: String,
    pub vector: Vec<f32>,
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl VectorRecord {
    pub fn parse_by_value(value: &serde_json::Value) -> Self {
        let id = value.get("id").unwrap().as_str().unwrap().to_string();
        let vector = value.get("vector").unwrap().as_array().unwrap().iter().map(|v| v.as_f64().unwrap() as f32).collect();
        let metadata = value.get("metadata").unwrap().as_object().unwrap().clone();
        Self { id, vector, metadata }
    }

    pub fn to_value(&self) -> serde_json::Value {
        json!({
            "id": self.id,
            "vector": self.vector,
            "metadata": self.metadata
        })
    }
}

fn qdrant_value_to_serde_json(q_val: QdrantValue) -> SerdeValue {
    match q_val.kind {
        Some(QdrantKind::NullValue(_)) => SerdeValue::Null,
        Some(QdrantKind::BoolValue(b)) => SerdeValue::Bool(b),
        Some(QdrantKind::DoubleValue(d)) => {
            SerdeNumber::from_f64(d).map_or(SerdeValue::Null, SerdeValue::Number)
        }
        Some(QdrantKind::IntegerValue(i)) => SerdeValue::Number(i.into()),
        Some(QdrantKind::StringValue(s)) => SerdeValue::String(s),
        Some(QdrantKind::ListValue(list_value)) => {
            let serde_list: Vec<SerdeValue> = list_value
                .values
                .into_iter()
                .map(qdrant_value_to_serde_json)
                .collect();
            SerdeValue::Array(serde_list)
        }
        Some(QdrantKind::StructValue(struct_value)) => {
            let mut serde_map = SerdeMap::new();
            for (key, val) in struct_value.fields {
                serde_map.insert(key, qdrant_value_to_serde_json(val));
            }
            SerdeValue::Object(serde_map)
        }
        None => SerdeValue::Null, // Treat absence of kind as Null
    }
}

impl VectorRecord {
    pub fn from_scored_point(point: ScoredPoint) -> Option<Self> {
        let id_str = match point.id {
            Some(point_id) => match point_id.point_id_options {
                Some(qdrant_client::qdrant::point_id::PointIdOptions::Num(n)) => n.to_string(),
                Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(s)) => s,
                None => return None,
            },
            None => return None,
        };
        let vector_data = match point.vectors {
            Some(vector) => match vector.vectors_options {
                Some(qdrant_client::qdrant::vectors_output::VectorsOptions::Vector(v)) => v.data,
                _ => return None,
            },
            None => return None,
        };
        // 3. Convert Payload
        let metadata_map: SerdeMap<String, SerdeValue> = point
            .payload
            .into_iter()
            .map(|(key, q_val)| (key, qdrant_value_to_serde_json(q_val)))
            .collect();

        Some(VectorRecord {
            id: id_str,
            vector: vector_data,
            metadata: metadata_map,
        })
    }
}

#[async_trait]
pub trait VectorDB {
    async fn insert(&self, records: Vec<VectorRecord>) -> anyhow::Result<()>;
    async fn search(&self, query: Vec<f32>, k: usize) -> anyhow::Result<Vec<VectorRecord>>;
    async fn delete(&self, ids: Vec<String>) -> anyhow::Result<()>;
}

pub struct QdrantDB {
    client: Qdrant,
    options: VectorDBOptions,
}

impl QdrantDB {
    pub async fn new(
        db_url: String,
        api_key: Option<String>,
        options: VectorDBOptions,
    ) -> anyhow::Result<Self> {
        let client = match api_key {
            Some(api_key) => Qdrant::from_url(db_url.as_str()).api_key(api_key).build()?,
            None => Qdrant::from_url(db_url.as_str()).build()?,
        };

        // Create collection if it doesn't exist
        let collections = client.list_collections().await?;
        if !collections
            .collections
            .iter()
            .any(|c| c.name == options.collection_name)
        {
            let distance = match options.distance_metric {
                DistanceMetric::Cosine => Distance::Cosine,
                DistanceMetric::Euclidean => Distance::Euclid,
                DistanceMetric::DotProduct => Distance::Dot,
            };
            let request = CreateCollectionBuilder::new(options.collection_name.clone())
                .vectors_config(VectorParamsBuilder::new(options.dimension as u64, distance));
            client.create_collection(request).await?;
        }

        Ok(Self { client, options })
    }
}

#[async_trait]
impl VectorDB for QdrantDB {
    async fn insert(&self, records: Vec<VectorRecord>) -> anyhow::Result<()> {
        let points: Vec<PointStruct> = records
            .into_iter()
            .map(|record| PointStruct::new(record.id, record.vector, record.metadata))
            .collect();
        let points_request = UpsertPointsBuilder::new(&self.options.collection_name, points);

        info!("Inserting points into Qdrant");
        self.client.upsert_points(points_request).await?;
        Ok(())
    }

    async fn search(&self, query: Vec<f32>, k: usize) -> anyhow::Result<Vec<VectorRecord>> {
        info!(
            "Searching points in Qdrant, collection: {}",
            self.options.collection_name
        );
        let response = self
            .client
            .search_points(
                SearchPointsBuilder::new(&self.options.collection_name, query, k as u64)
                    .with_payload(true)
                    .with_vectors(true),
            )
            .await?;
        let results = response
            .result
            .into_iter()
            .filter_map(VectorRecord::from_scored_point)
            .collect::<Vec<_>>();
        info!("Retrieved results len: {:?}", results.len());

        Ok(results)
    }

    async fn delete(&self, ids: Vec<String>) -> anyhow::Result<()> {
        info!("Deleting points from Qdrant");
        self.client
            .delete_points(DeletePointsBuilder::new(&self.options.collection_name).points(ids))
            .await?;
        Ok(())
    }
}
