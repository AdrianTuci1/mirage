use crate::config::DaemonConfig;
use anyhow::{Context, Result};
use arrow_schema::DataType;
use duckdb::{Connection, Row};
use serde_json::{Map, Number, Value};
use std::path::{Path, PathBuf};

/// Embedded DuckDB analytics engine.
///
/// The database lives inside `data_dir/analytics.duckdb` and is bundled with the daemon binary,
/// matching the project requirement that DuckDB is pre-installed and app-local.
pub struct Analytics {
    db_path: PathBuf,
}

impl Analytics {
    pub fn open(config: &DaemonConfig) -> Result<Self> {
        let db_path = config.data_dir.join("analytics.duckdb");
        std::fs::create_dir_all(&config.data_dir)
            .with_context(|| format!("failed to create data directory {}", config.data_dir.display()))?;

        let conn = Connection::open(&db_path)
            .with_context(|| format!("failed to open DuckDB at {}", db_path.display()))?;
        // Ensure the file is created and accessible.
        drop(conn);

        Ok(Self { db_path })
    }

    fn connect(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
            .with_context(|| format!("failed to open DuckDB at {}", self.db_path.display()))
    }

    /// Execute an arbitrary SQL statement that does not return rows (DDL/DML).
    /// Returns the number of rows changed for INSERT/UPDATE/DELETE.
    pub fn execute(&self, sql: &str) -> Result<usize> {
        let conn = self.connect()?;
        let rows = conn.execute(sql, []).context("failed to execute SQL")?;
        Ok(rows)
    }

    /// Execute a SELECT query and return rows as JSON values.
    pub fn query(&self, sql: &str) -> Result<Vec<Map<String, Value>>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(sql).context("failed to prepare SQL query")?;

        // DuckDB only exposes column metadata after the statement has been executed.
        stmt.execute([]).context("failed to execute query for metadata")?;
        let column_count = stmt.column_count();
        let column_names: Vec<String> = (0..column_count)
            .map(|i| stmt.column_name(i).map_or_else(|_| "?".to_string(), |s| s.to_string()))
            .collect();
        let column_types: Vec<DataType> = (0..column_count)
            .map(|i| stmt.column_type(i))
            .collect();

        let mut rows = stmt.query([]).context("failed to execute query")?;
        let mut results = Vec::new();
        while let Some(row) = rows.next().context("failed to read query row")? {
            results.push(row_to_json(row, &column_names, &column_types)?);
        }
        Ok(results)
    }

    /// Load or replace a table from a CSV file.
    pub fn ingest_csv(&self, path: impl AsRef<Path>, table_name: &str) -> Result<usize> {
        let path = path.as_ref();
        let escaped = escape_sql_identifier(table_name);
        let path_str = path.to_string_lossy();
        let sql = format!(
            "CREATE OR REPLACE TABLE {} AS SELECT * FROM read_csv_auto('{}')",
            escaped,
            escape_sql_string(&path_str)
        );
        let conn = self.connect()?;
        let rows = conn
            .execute(&sql, [])
            .with_context(|| format!("failed to ingest CSV {}", path.display()))?;
        Ok(rows)
    }

    /// Load or replace a table from a Parquet file.
    pub fn ingest_parquet(&self, path: impl AsRef<Path>, table_name: &str) -> Result<usize> {
        let path = path.as_ref();
        let escaped = escape_sql_identifier(table_name);
        let path_str = path.to_string_lossy();
        let sql = format!(
            "CREATE OR REPLACE TABLE {} AS SELECT * FROM read_parquet('{}')",
            escaped,
            escape_sql_string(&path_str)
        );
        let conn = self.connect()?;
        let rows = conn
            .execute(&sql, [])
            .with_context(|| format!("failed to ingest Parquet {}", path.display()))?;
        Ok(rows)
    }

    /// Expose the database path for advanced callers that need a raw connection.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

fn row_to_json(row: &Row, column_names: &[String], column_types: &[DataType]) -> Result<Map<String, Value>> {
    let mut map = Map::new();
    for (idx, name) in column_names.iter().enumerate() {
        let value = match column_types.get(idx) {
            Some(DataType::Int8)
            | Some(DataType::Int16)
            | Some(DataType::Int32)
            | Some(DataType::Int64)
            | Some(DataType::UInt8)
            | Some(DataType::UInt16)
            | Some(DataType::UInt32)
            | Some(DataType::UInt64) => row
                .get::<_, Option<i64>>(idx)
                .map(|v| v.map(|n| Value::Number(n.into())).unwrap_or(Value::Null))
                .unwrap_or(Value::Null),
            Some(DataType::Float32) | Some(DataType::Float64) => row
                .get::<_, Option<f64>>(idx)
                .map(|v| {
                    v.and_then(|n| Number::from_f64(n).map(Value::Number))
                        .unwrap_or(Value::Null)
                })
                .unwrap_or(Value::Null),
            Some(DataType::Utf8) | Some(DataType::LargeUtf8) => row
                .get::<_, Option<String>>(idx)
                .map(|v| v.map(Value::String).unwrap_or(Value::Null))
                .unwrap_or(Value::Null),
            Some(DataType::Boolean) => row
                .get::<_, Option<bool>>(idx)
                .map(|v| v.map(Value::Bool).unwrap_or(Value::Null))
                .unwrap_or(Value::Null),
            _ => {
                // Fallback to string representation for complex DuckDB types.
                let repr = row
                    .get_ref(idx)
                    .map(|r| format!("{:?}", r))
                    .unwrap_or_else(|_| "null".to_string());
                Value::String(repr)
            }
        };
        map.insert(name.clone(), value);
    }
    Ok(map)
}

fn escape_sql_identifier(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

fn escape_sql_string(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_analytics() -> (Analytics, TempDir) {
        let dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            data_dir: dir.path().to_path_buf(),
            ..DaemonConfig::default()
        };
        (Analytics::open(&config).unwrap(), dir)
    }

    #[test]
    fn create_and_query_table() {
        let (analytics, _dir) = temp_analytics();
        analytics
            .execute("CREATE TABLE numbers (id INTEGER, value DOUBLE)")
            .unwrap();
        analytics
            .execute("INSERT INTO numbers VALUES (1, 10.5), (2, 20.0)")
            .unwrap();

        let rows = analytics.query("SELECT * FROM numbers ORDER BY id").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["id"], Value::Number(1_i64.into()));
        assert_eq!(
            rows[1]["value"],
            Value::Number(Number::from_f64(20.0).unwrap())
        );
    }

    #[test]
    fn query_builtin_constants() {
        let (analytics, _dir) = temp_analytics();
        let rows = analytics.query("SELECT 1 AS one, 'hello' AS msg").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["one"], Value::Number(1_i64.into()));
        assert_eq!(rows[0]["msg"], Value::String("hello".to_string()));
    }

    #[test]
    fn csv_ingest_works() {
        let (analytics, dir) = temp_analytics();
        let csv_path = dir.path().join("data.csv");
        std::fs::write(&csv_path, "name,score\nAlice,42\nBob,7\n").unwrap();

        analytics.ingest_csv(&csv_path, "people").unwrap();
        let rows = analytics
            .query("SELECT * FROM people ORDER BY score DESC")
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], Value::String("Alice".to_string()));
    }
}
