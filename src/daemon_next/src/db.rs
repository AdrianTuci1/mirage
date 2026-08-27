use crate::config::DaemonConfig;
use crate::models::{Record, RecordWithScore};
use anyhow::{Context, Result};
use arrow_array::types::Float32Type;
use arrow_array::{
    Array, Float32Array, Int64Array, ListArray, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use futures::stream::StreamExt;
use lancedb::query::ExecutableQuery;
use lancedb::{connect, Connection, Table};
use std::sync::Arc;

const TABLE_NAME: &str = "records";
const VECTOR_COLUMN: &str = "vector";

pub struct LanceDbStore {
    _db: Connection,
    table: Table,
}

impl LanceDbStore {
    pub async fn open(config: &DaemonConfig) -> Result<Self> {
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
            let schema = Self::schema();
            let batches =
                RecordBatchIterator::new(vec![].into_iter().map(Ok), schema.clone());
            db.create_table(
                TABLE_NAME,
                Box::new(batches) as Box<dyn arrow_array::RecordBatchReader + Send>,
            )
            .execute()
            .await
            .context("failed to create records table")?
        };

        Ok(Self { _db: db, table })
    }

    pub async fn upsert(&self, records: Vec<Record>) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        let ids: Vec<String> = records.iter().map(|r| r.id.clone()).collect();
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

        let batch =
            Self::record_batch_from_records(&records).context("failed to build record batch")?;
        self.table
            .add(vec![batch])
            .execute()
            .await
            .context("failed to add records")?;

        Ok(())
    }

    pub async fn search(
        &self,
        query_vector: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<RecordWithScore>> {
        let records = self.all_records().await?;
        let mut scored: Vec<RecordWithScore> = records
            .into_iter()
            .map(|record| {
                let score = cosine_similarity(&query_vector, &record.vector);
                RecordWithScore { record, score }
            })
            .collect();
        scored.sort_by(
            |a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal),
        );
        scored.truncate(top_k);
        Ok(scored)
    }

    pub async fn count(&self) -> Result<usize> {
        let count = self
            .table
            .count_rows(None)
            .await
            .context("failed to count records")?;
        Ok(count)
    }

    fn schema() -> SchemaRef {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("relative_path", DataType::Utf8, false),
            Field::new("source_type", DataType::Utf8, false),
            Field::new(
                VECTOR_COLUMN,
                DataType::List(Arc::new(Field::new("item", DataType::Float32, true))),
                false,
            ),
            Field::new("updated_at", DataType::Utf8, false),
            Field::new("version", DataType::Int64, false),
        ]))
    }

    fn record_batch_from_records(records: &[Record]) -> Result<RecordBatch> {
        let schema = Self::schema();
        let ids = StringArray::from_iter_values(records.iter().map(|r| r.id.clone()));
        let paths = StringArray::from_iter_values(records.iter().map(|r| r.relative_path.clone()));
        let sources = StringArray::from_iter_values(records.iter().map(|r| r.source_type.clone()));
        let updated_ats =
            StringArray::from_iter_values(records.iter().map(|r| r.updated_at.clone()));
        let versions = Int64Array::from_iter_values(records.iter().map(|r| r.version));

        let vector_array = ListArray::from_iter_primitive::<Float32Type, _, _>(records.iter().map(
            |r| {
                let values: Vec<Option<f32>> = r.vector.iter().map(|&v| Some(v)).collect();
                Some(values)
            },
        ));

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

    fn records_from_batch(batch: &RecordBatch) -> Result<Vec<Record>> {
        let id_col = batch
            .column_by_name("id")
            .context("missing id column")?
            .as_any()
            .downcast_ref::<StringArray>()
            .context("id column is not utf8")?;
        let path_col = batch
            .column_by_name("relative_path")
            .context("missing relative_path column")?
            .as_any()
            .downcast_ref::<StringArray>()
            .context("relative_path column is not utf8")?;
        let source_col = batch
            .column_by_name("source_type")
            .context("missing source_type column")?
            .as_any()
            .downcast_ref::<StringArray>()
            .context("source_type column is not utf8")?;
        let vector_col = batch
            .column_by_name(VECTOR_COLUMN)
            .context("missing vector column")?
            .as_any()
            .downcast_ref::<ListArray>()
            .context("vector column is not list")?;
        let updated_col = batch
            .column_by_name("updated_at")
            .context("missing updated_at column")?
            .as_any()
            .downcast_ref::<StringArray>()
            .context("updated_at column is not utf8")?;
        let version_col = batch
            .column_by_name("version")
            .context("missing version column")?
            .as_any()
            .downcast_ref::<Int64Array>()
            .context("version column is not int64")?;

        let mut records = Vec::new();
        for i in 0..batch.num_rows() {
            let values = vector_col.value(i);
            let float_array = values
                .as_any()
                .downcast_ref::<Float32Array>()
                .context("vector values are not float32")?;
            let vector: Vec<f32> = float_array.values().to_vec();

            records.push(Record {
                id: id_col.value(i).to_string(),
                relative_path: path_col.value(i).to_string(),
                source_type: source_col.value(i).to_string(),
                vector,
                updated_at: updated_col.value(i).to_string(),
                version: version_col.value(i),
            });
        }
        Ok(records)
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0_f64;
    let mut norm_a = 0.0_f64;
    let mut norm_b = 0.0_f64;
    for i in 0..a.len() {
        let av = a[i] as f64;
        let bv = b[i] as f64;
        dot += av * bv;
        norm_a += av * av;
        norm_b += bv * bv;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a.sqrt() * norm_b.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_equal_vectors_is_one() {
        let v = vec![1.0_f32, 2.0, 3.0];
        let score = cosine_similarity(&v, &v);
        assert!(score > 0.999 && score < 1.001, "expected ~1.0, got {}", score);
    }

    #[test]
    fn cosine_similarity_orthogonal_is_zero() {
        let a = vec![1.0_f32, 0.0];
        let b = vec![0.0_f32, 1.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }
}
