use crate::config::DaemonConfig;
use crate::models::{Record, RecordWithScore};
use anyhow::{Context, Result};
use arrow_array::types::Float32Type;
use arrow_array::{
    Array, FixedSizeListArray, Float32Array, Int64Array, RecordBatch, RecordBatchIterator,
    StringArray,
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use futures::stream::StreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::{connect, Connection, Table};
use std::sync::Arc;

const TABLE_NAME: &str = "records";
const VECTOR_COLUMN: &str = "vector";

pub struct LanceDbStore {
    _db: Connection,
    table: Table,
    dimension: usize,
}

impl LanceDbStore {
    pub async fn open(config: &DaemonConfig) -> Result<Self> {
        Self::open_with_dimension(config, config.vector_dim).await
    }

    pub async fn open_with_dimension(config: &DaemonConfig, dimension: usize) -> Result<Self> {
        let db_path = config.data_dir.join("lancedb");
        tokio::fs::create_dir_all(&db_path)
            .await
            .with_context(|| format!("failed to create LanceDB directory {}", db_path.display()))?;

        let uri = db_path.to_string_lossy().to_string();
        let db = connect(&uri)
            .execute()
            .await
            .with_context(|| format!("failed to open LanceDB at {}", uri))?;

        let table_names = db
            .table_names()
            .execute()
            .await
            .context("failed to list table names")?;
        let table = if table_names.iter().any(|n| n == TABLE_NAME) {
            db.open_table(TABLE_NAME)
                .execute()
                .await
                .context("failed to open records table")?
        } else {
            let schema = Self::schema(dimension);
            let batches = RecordBatchIterator::new(vec![].into_iter().map(Ok), schema.clone());
            db.create_table(
                TABLE_NAME,
                Box::new(batches) as Box<dyn arrow_array::RecordBatchReader + Send>,
            )
            .execute()
            .await
            .context("failed to create records table")?
        };

        Ok(Self {
            _db: db,
            table,
            dimension,
        })
    }

    pub async fn upsert(&self, records: Vec<Record>) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        let batch = Self::record_batch_from_records(&records, self.dimension)
            .context("failed to build record batch")?;
        self.table
            .add(vec![batch])
            .execute()
            .await
            .context("failed to add records")?;

        Ok(())
    }

    /// Upsert records in batches. For each batch, existing records with matching ids are deleted
    /// before the new batch is inserted. This keeps memory usage bounded.
    pub async fn upsert_batched(&self, records: Vec<Record>, batch_size: usize) -> Result<()> {
        let batch_size = batch_size.max(1);
        for chunk in records.chunks(batch_size) {
            let ids: Vec<String> = chunk.iter().map(|r| r.id.clone()).collect();
            let id_list = ids
                .iter()
                .map(|id| format!("'{}'", id.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(", ");
            let filter = format!("id IN ({})", id_list);
            self.table
                .delete(&filter)
                .await
                .context("failed to delete existing records before upsert")?;

            self.upsert(chunk.to_vec()).await?;
        }
        Ok(())
    }

    pub async fn search(
        &self,
        query_vector: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<RecordWithScore>> {
        let mut results = self
            .table
            .vector_search(&*query_vector)
            .map_err(|e| anyhow::anyhow!("failed to start vector search: {}", e))?
            .column(VECTOR_COLUMN)
            .distance_type(lancedb::DistanceType::Cosine)
            .limit(top_k)
            .execute()
            .await
            .context("failed to execute vector search")?;

        let mut records = Vec::new();
        while let Some(batch) = results.next().await {
            let batch = batch.context("failed to read search result batch")?;
            let batch_records = Self::scored_records_from_batch(&batch)?;
            records.extend(batch_records);
        }
        Ok(records)
    }

    pub async fn count(&self) -> Result<usize> {
        let count = self
            .table
            .count_rows(None)
            .await
            .context("failed to count records")?;
        Ok(count)
    }

    /// Delete all records whose `source_type` matches the supplied value.
    pub async fn delete_by_source_type(&self, source_type: &str) -> Result<()> {
        let escaped = source_type.replace('\'', "''");
        let filter = format!("source_type = '{}'", escaped);
        self.table
            .delete(&filter)
            .await
            .context("failed to delete records by source_type")?;
        Ok(())
    }

    fn schema(dimension: usize) -> SchemaRef {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("relative_path", DataType::Utf8, false),
            Field::new("source_type", DataType::Utf8, false),
            Field::new(
                VECTOR_COLUMN,
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dimension as i32,
                ),
                false,
            ),
            Field::new("updated_at", DataType::Utf8, false),
            Field::new("version", DataType::Int64, false),
        ]))
    }

    fn record_batch_from_records(records: &[Record], dimension: usize) -> Result<RecordBatch> {
        let schema = Self::schema(dimension);
        let ids = StringArray::from_iter_values(records.iter().map(|r| r.id.clone()));
        let paths = StringArray::from_iter_values(records.iter().map(|r| r.relative_path.clone()));
        let sources = StringArray::from_iter_values(records.iter().map(|r| r.source_type.clone()));
        let updated_ats =
            StringArray::from_iter_values(records.iter().map(|r| r.updated_at.clone()));
        let versions = Int64Array::from_iter_values(records.iter().map(|r| r.version));

        let vector_array = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
            records.iter().map(|r| {
                let values: Vec<Option<f32>> = (0..dimension)
                    .map(|i| r.vector.get(i).copied().or(Some(0.0_f32)))
                    .collect();
                Some(values)
            }),
            dimension as i32,
        );

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(ids),
                Arc::new(paths),
                Arc::new(sources),
                Arc::new(vector_array),
                Arc::new(updated_ats),
                Arc::new(versions),
            ],
        )
        .context("failed to build record batch from records")
    }

    async fn all_records(&self) -> Result<Vec<Record>> {
        let mut results = self
            .table
            .query()
            .execute()
            .await
            .context("failed to query all records")?;

        let mut records = Vec::new();
        while let Some(batch) = results.next().await {
            let batch = batch.context("failed to read record batch")?;
            let batch_records = Self::records_from_batch(&batch)?;
            records.extend(batch_records);
        }
        Ok(records)
    }

    fn scored_records_from_batch(batch: &RecordBatch) -> Result<Vec<RecordWithScore>> {
        let records = Self::records_from_batch(batch)?;
        let distances = batch
            .column_by_name("_distance")
            .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

        let mut scored = Vec::new();
        for (idx, record) in records.into_iter().enumerate() {
            let score = distances
                .map(|d| (1.0 - d.value(idx)) as f64)
                .unwrap_or(0.0);
            scored.push(RecordWithScore { record, score });
        }
        Ok(scored)
    }

    fn records_from_batch(batch: &RecordBatch) -> Result<Vec<Record>> {
        let ids = batch
            .column_by_name("id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .context("missing id column")?;
        let relative_paths = batch
            .column_by_name("relative_path")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .context("missing relative_path column")?;
        let source_types = batch
            .column_by_name("source_type")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .context("missing source_type column")?;
        let vectors = batch
            .column_by_name(VECTOR_COLUMN)
            .and_then(|c| c.as_any().downcast_ref::<FixedSizeListArray>())
            .context("missing vector column")?;
        let updated_ats = batch
            .column_by_name("updated_at")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .context("missing updated_at column")?;
        let versions = batch
            .column_by_name("version")
            .and_then(|c| c.as_any().downcast_ref::<Int64Array>())
            .context("missing version column")?;

        let mut records = Vec::new();
        for i in 0..batch.num_rows() {
            let vector =
                if let Some(values) = vectors.value(i).as_any().downcast_ref::<Float32Array>() {
                    (0..vectors.value_length())
                        .map(|j| values.value(j as usize))
                        .collect()
                } else {
                    Vec::new()
                };
            records.push(Record {
                id: ids.value(i).to_string(),
                relative_path: relative_paths.value(i).to_string(),
                source_type: source_types.value(i).to_string(),
                vector,
                updated_at: updated_ats.value(i).to_string(),
                version: versions.value(i),
            });
        }
        Ok(records)
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0_f32;
    let mut norm_a = 0.0_f32;
    let mut norm_b = 0.0_f32;
    for i in 0..a.len().min(b.len()) {
        let av = a[i];
        let bv = b[i];
        dot += av * bv;
        norm_a += av * av;
        norm_b += bv * bv;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a.sqrt() * norm_b.sqrt())
}

/// Reduce or pad a vector to a target dimension.
///
/// Longer vectors are downsampled by averaging adjacent values; shorter vectors
/// are padded with zeros. The result is re-normalized to keep cosine distance
/// meaningful.
pub fn downsample_vector(v: &[f32], target_dim: usize) -> Vec<f32> {
    if target_dim == 0 {
        return Vec::new();
    }
    if v.len() == target_dim {
        return v.to_vec();
    }

    let mut out = Vec::with_capacity(target_dim);
    if v.is_empty() {
        out.resize(target_dim, 0.0_f32);
        return out;
    }

    if v.len() < target_dim {
        out.extend_from_slice(v);
        out.resize(target_dim, 0.0_f32);
        return normalize(&mut out);
    }

    let window = v.len() / target_dim;
    let extra = v.len() % target_dim;
    let mut start = 0;
    for i in 0..target_dim {
        let width = window + if i < extra { 1 } else { 0 };
        let end = (start + width).min(v.len());
        let slice = &v[start..end];
        let avg = slice.iter().sum::<f32>() / slice.len() as f32;
        out.push(avg);
        start = end;
    }
    normalize(&mut out)
}

fn normalize(v: &mut [f32]) -> Vec<f32> {
    let sum_sq: f32 = v.iter().map(|x| x * x).sum();
    if sum_sq == 0.0 {
        return v.to_vec();
    }
    let norm = sum_sq.sqrt();
    v.iter_mut().for_each(|x| *x /= norm);
    v.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_config() -> (DaemonConfig, TempDir) {
        let dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            data_dir: dir.path().to_path_buf(),
            vector_dim: 3,
            ..DaemonConfig::default()
        };
        (config, dir)
    }

    #[tokio::test]
    async fn cosine_similarity_equal_vectors_is_one() {
        let (config, _dir) = temp_config();
        let store = LanceDbStore::open(&config).await.unwrap();
        let v = vec![1.0_f32, 0.0_f32, 0.0_f32];
        let records = vec![Record {
            id: "r1".to_string(),
            relative_path: "/a".to_string(),
            source_type: "local".to_string(),
            vector: v.clone(),
            updated_at: "2024-01-01".to_string(),
            version: 1,
        }];
        store.upsert(records).await.unwrap();

        let results = store.search(v.clone(), 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(
            (results[0].score - 1.0).abs() < 0.01,
            "score should be ~1.0"
        );
    }

    #[tokio::test]
    async fn cosine_similarity_orthogonal_is_zero() {
        let (config, _dir) = temp_config();
        let store = LanceDbStore::open(&config).await.unwrap();
        let records = vec![Record {
            id: "r1".to_string(),
            relative_path: "/a".to_string(),
            source_type: "local".to_string(),
            vector: vec![1.0_f32, 0.0_f32, 0.0_f32],
            updated_at: "2024-01-01".to_string(),
            version: 1,
        }];
        store.upsert(records).await.unwrap();

        let results = store
            .search(vec![0.0_f32, 1.0_f32, 0.0_f32], 10)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].score.abs() < 0.01, "score should be ~0.0");
    }

    #[test]
    fn downsample_preserves_equal_dimension() {
        let v = vec![1.0_f32, 0.0_f32, 0.0_f32];
        let out = super::downsample_vector(&v, 3);
        assert_eq!(out, v);
    }

    #[test]
    fn downsample_pads_short_vectors() {
        let v = vec![1.0_f32, 0.0_f32];
        let out = super::downsample_vector(&v, 4);
        assert_eq!(out.len(), 4);
        assert!((out[0] - 1.0).abs() < 0.01);
        assert!(out[1].abs() < 0.01);
    }

    #[test]
    fn downsample_reduces_long_vectors() {
        let v: Vec<f32> = (0..8).map(|i| i as f32).collect();
        let out = super::downsample_vector(&v, 4);
        assert_eq!(out.len(), 4);
        // Averages [0.5, 2.5, 4.5, 6.5] are re-normalized before returning.
        let norm = (0.5_f32 * 0.5 + 2.5 * 2.5 + 4.5 * 4.5 + 6.5 * 6.5).sqrt();
        let expected = vec![0.5 / norm, 2.5 / norm, 4.5 / norm, 6.5 / norm];
        for (a, b) in out.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 0.01);
        }
    }
}
