use crate::config::DaemonConfig;
use anyhow::{anyhow, Context, Result};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

/// Id of the runtime module that provides the SQL engine.
pub const DUCKDB_MODULE_ID: &str = "duckdb";

/// Optional override pointing at a DuckDB CLI already installed outside the
/// module directory. Used by developers and by the test suite.
pub const DUCKDB_BIN_ENV: &str = "MIRAGE_DUCKDB_BIN";

const MISSING_ENGINE: &str =
    "DuckDB analytics module is not installed. Download it via the Modules settings.";

/// Tabular analytics through the downloadable DuckDB engine.
///
/// The engine is the official DuckDB command-line binary, installed as the
/// `duckdb` runtime module under `<downloads_dir>/duckdb/<version>/`. Each
/// statement is one invocation of
/// `<engine> -json -batch -bail -no-init <db_path> -c <sql>` whose JSON result
/// set becomes rows. The daemon never links DuckDB itself, so the same binary
/// serves the tabular feature with or without the engine present; the only
/// difference is [`Analytics::is_available`].
pub struct Analytics {
    db_path: PathBuf,
    /// `<downloads_dir>/duckdb`, i.e. the module directory holding version dirs.
    engine_dir: PathBuf,
    engine_override: Option<PathBuf>,
    /// DuckDB gives one process an exclusive lock on the database file, so
    /// engine invocations are serialized here instead of failing on it.
    call_lock: Mutex<()>,
}

impl Analytics {
    pub fn open(config: &DaemonConfig) -> Result<Self> {
        std::fs::create_dir_all(&config.data_dir).with_context(|| {
            format!(
                "failed to create data directory {}",
                config.data_dir.display()
            )
        })?;

        Ok(Self {
            db_path: config.data_dir.join("analytics.duckdb"),
            engine_dir: config.downloads_dir.join(DUCKDB_MODULE_ID),
            engine_override: engine_override(),
            call_lock: Mutex::new(()),
        })
    }

    /// Path of the engine binary, or `None` while the module is not installed.
    /// Resolved per call so a download that finishes after startup is picked up
    /// without restarting the daemon.
    pub fn engine_binary(&self) -> Option<PathBuf> {
        if let Some(path) = &self.engine_override {
            if path.is_file() {
                return Some(path.clone());
            }
        }
        installed_engine(&self.engine_dir)
    }

    /// The SQL engine can answer queries in this build.
    pub fn is_available(&self) -> bool {
        self.engine_binary().is_some()
    }

    /// Execute a statement that does not return rows (DDL/DML).
    ///
    /// The DuckDB CLI reports no affected-row count, so this only signals
    /// success or failure.
    pub fn execute(&self, sql: &str) -> Result<()> {
        self.invoke(sql).map(|_| ())
    }

    /// Execute a query and return its rows as JSON objects.
    pub fn query(&self, sql: &str) -> Result<Vec<Map<String, Value>>> {
        self.invoke(sql)?
            .into_iter()
            .map(|row| {
                row.as_object().cloned().ok_or_else(|| {
                    anyhow!("the DuckDB engine returned a row that is not an object")
                })
            })
            .collect()
    }

    /// Load or replace a table from a CSV file. Returns the rows loaded.
    pub fn ingest_csv(&self, path: impl AsRef<Path>, table_name: &str) -> Result<usize> {
        self.ingest(table_name, "read_csv_auto", path.as_ref())
    }

    /// Load or replace a table from a Parquet file. Returns the rows loaded.
    pub fn ingest_parquet(&self, path: impl AsRef<Path>, table_name: &str) -> Result<usize> {
        self.ingest(table_name, "read_parquet", path.as_ref())
    }

    /// Expose the database path for callers that need a raw file location.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    fn ingest(&self, table_name: &str, reader: &str, path: &Path) -> Result<usize> {
        let escaped = escape_sql_identifier(table_name);
        let sql = format!(
            "CREATE OR REPLACE TABLE {escaped} AS SELECT * FROM {reader}('{}'); SELECT count(*) AS row_count FROM {escaped}",
            escape_sql_string(&path.to_string_lossy())
        );
        let mut rows = self
            .invoke(&sql)
            .with_context(|| format!("failed to ingest {} with {reader}", path.display()))?;
        let count = match rows.pop() {
            Some(row) => row
                .get("row_count")
                .and_then(Value::as_u64)
                .unwrap_or_default(),
            None => 0,
        };
        Ok(count as usize)
    }

    /// Run one engine invocation and return the first result set.
    fn invoke(&self, sql: &str) -> Result<Vec<Value>> {
        let engine = self
            .engine_binary()
            .ok_or_else(|| anyhow!(MISSING_ENGINE))?;
        let _guard = self
            .call_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let output = Command::new(&engine)
            .arg("-json")
            .arg("-batch")
            .arg("-bail")
            .arg("-no-init")
            .arg(&self.db_path)
            .arg("-c")
            .arg(sql)
            .stdin(Stdio::null())
            .output()
            .with_context(|| {
                format!("failed to launch the DuckDB engine at {}", engine.display())
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let reason = stderr
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("the DuckDB engine failed");
            return Err(anyhow!("duckdb: {reason}"));
        }

        let stdout = String::from_utf8(output.stdout)
            .context("the DuckDB engine returned output that is not valid UTF-8")?;
        first_result_set(&stdout)
    }
}

/// The engine configured through [`DUCKDB_BIN_ENV`], when that variable points
/// at a file. Lets a developer or a test reuse a locally installed engine
/// instead of downloading the module.
pub fn engine_override() -> Option<PathBuf> {
    let path = std::env::var_os(DUCKDB_BIN_ENV).map(PathBuf::from)?;
    if path.is_file() {
        Some(path)
    } else {
        tracing::warn!(
            "{DUCKDB_BIN_ENV} points at {}, which is not a file; tabular analytics ignores it",
            path.display()
        );
        None
    }
}

/// Locate the engine binary inside a module directory laid out as
/// `<module_dir>/<version>/duckdb[.exe]`. Several versions are unusual — the
/// manager installs one directory per version — so the last in lexical order
/// wins.
pub fn installed_engine(module_dir: &Path) -> Option<PathBuf> {
    let mut versions: Vec<PathBuf> = std::fs::read_dir(module_dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|dir| dir.is_dir() && engine_executable(dir).is_some())
        .collect();
    versions.sort();
    versions.pop().as_deref().and_then(engine_executable)
}

/// The engine binary inside one module version directory.
fn engine_executable(version_dir: &Path) -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "duckdb.exe"
    } else {
        "duckdb"
    };
    let path = version_dir.join(name);
    path.is_file().then_some(path)
}

/// Read the first JSON array of the engine's stdout. Statements that produce no
/// result set print nothing, which yields an empty vector.
fn first_result_set(stdout: &str) -> Result<Vec<Value>> {
    let mut sets = serde_json::Deserializer::from_str(stdout).into_iter::<Value>();
    let Some(set) = sets.next() else {
        return Ok(Vec::new());
    };
    let set = set.context("failed to parse the JSON emitted by the DuckDB engine")?;
    match set {
        Value::Array(rows) => Ok(rows),
        other => Err(anyhow!(
            "expected a JSON array from the DuckDB engine, got {other}"
        )),
    }
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

    /// Engine-backed analytics, or `None` when no DuckDB CLI is installed.
    fn engine_analytics() -> Option<(Analytics, tempfile::TempDir)> {
        let dir = tempfile::TempDir::new().unwrap();
        let config = DaemonConfig {
            data_dir: dir.path().to_path_buf(),
            downloads_dir: dir.path().join("downloads"),
            ..DaemonConfig::default()
        };
        let analytics = Analytics::open(&config).unwrap();
        analytics.is_available().then_some((analytics, dir))
    }

    /// Analytics that behave as if no engine were installed, whatever the
    /// environment says: `MIRAGE_DUCKDB_BIN` is set for most of the suite.
    fn without_engine(config: &DaemonConfig) -> Analytics {
        let mut analytics = Analytics::open(config).unwrap();
        analytics.engine_override = None;
        analytics.engine_dir = config.downloads_dir.join("no-such-module");
        analytics
    }

    #[test]
    fn unavailable_without_the_downloaded_engine() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = DaemonConfig {
            data_dir: dir.path().to_path_buf(),
            downloads_dir: dir.path().join("empty-downloads"),
            ..DaemonConfig::default()
        };
        let analytics = without_engine(&config);

        assert!(!analytics.is_available());
        let err = analytics.query("SELECT 1").unwrap_err().to_string();
        assert!(
            err.contains("not installed"),
            "the error should tell the user to install the module, got {err}"
        );
    }

    #[test]
    fn the_engine_is_picked_up_from_a_module_version_directory() {
        let dir = tempfile::TempDir::new().unwrap();
        let engine_dir = dir.path().join("downloads").join(DUCKDB_MODULE_ID);
        let binary = format!("duckdb{}", if cfg!(windows) { ".exe" } else { "" });
        std::fs::create_dir_all(engine_dir.join("1.4.0")).unwrap();
        std::fs::create_dir_all(engine_dir.join("1.5.5")).unwrap();
        std::fs::File::create(engine_dir.join("1.4.0").join(&binary)).unwrap();
        let newest = engine_dir.join("1.5.5").join(&binary);
        std::fs::File::create(&newest).unwrap();

        let config = DaemonConfig {
            data_dir: dir.path().to_path_buf(),
            downloads_dir: dir.path().join("downloads"),
            ..DaemonConfig::default()
        };
        let mut analytics = Analytics::open(&config).unwrap();
        analytics.engine_override = None;

        assert_eq!(analytics.engine_binary(), Some(newest));
        assert!(analytics.is_available());

        // Removing the version directory makes the feature unavailable again,
        // without a restart: the path is resolved on every call.
        std::fs::remove_dir_all(&engine_dir).unwrap();
        assert!(!analytics.is_available());
    }

    #[test]
    fn create_and_query_table() {
        let Some((analytics, _dir)) = engine_analytics() else {
            eprintln!("skipping: no DuckDB engine installed ({DUCKDB_BIN_ENV} unset)");
            return;
        };
        analytics
            .execute("CREATE TABLE numbers (id INTEGER, value DOUBLE)")
            .unwrap();
        analytics
            .execute("INSERT INTO numbers VALUES (1, 10.5), (2, 20.0)")
            .unwrap();

        let rows = analytics
            .query("SELECT * FROM numbers ORDER BY id")
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["id"], Value::Number(1_i64.into()));
        assert_eq!(
            rows[1]["value"],
            Value::Number(serde_json::Number::from_f64(20.0).unwrap())
        );
    }

    #[test]
    fn query_builtin_constants() {
        let Some((analytics, _dir)) = engine_analytics() else {
            eprintln!("skipping: no DuckDB engine installed ({DUCKDB_BIN_ENV} unset)");
            return;
        };
        let rows = analytics.query("SELECT 1 AS one, 'hello' AS msg").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["one"], Value::Number(1_i64.into()));
        assert_eq!(rows[0]["msg"], Value::String("hello".to_string()));
    }

    #[test]
    fn csv_ingest_works() {
        let Some((analytics, dir)) = engine_analytics() else {
            eprintln!("skipping: no DuckDB engine installed ({DUCKDB_BIN_ENV} unset)");
            return;
        };
        let csv_path = dir.path().join("data.csv");
        std::fs::write(&csv_path, "name,score\nAlice,42\nBob,7\n").unwrap();

        assert_eq!(analytics.ingest_csv(&csv_path, "people").unwrap(), 2);
        let rows = analytics
            .query("SELECT * FROM people ORDER BY score DESC")
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], Value::String("Alice".to_string()));
    }

    #[test]
    fn a_failing_statement_is_an_error_not_an_empty_result() {
        let Some((analytics, _dir)) = engine_analytics() else {
            eprintln!("skipping: no DuckDB engine installed ({DUCKDB_BIN_ENV} unset)");
            return;
        };
        // The CLI prints "[]" and writes the message to stderr, so only the exit
        // status can tell a failed query apart from a genuinely empty one.
        let err = analytics.query("SELECT * FROM table_that_does_not_exist");
        assert!(err.is_err(), "a catalog error must surface to the caller");
        assert!(err.unwrap_err().to_string().contains("duckdb:"));
    }

    #[test]
    fn an_empty_result_set_is_not_an_error() {
        let Some((analytics, _dir)) = engine_analytics() else {
            eprintln!("skipping: no DuckDB engine installed ({DUCKDB_BIN_ENV} unset)");
            return;
        };
        assert!(analytics
            .query("SELECT 1 AS one WHERE false")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn sql_string_escaping_survives_the_round_trip() {
        assert_eq!(escape_sql_string("it's"), "it''s");
        assert_eq!(escape_sql_identifier("we\"ird"), "\"we\"\"ird\"");
    }

    #[test]
    fn result_sets_are_read_from_the_engine_output() {
        assert!(first_result_set("").unwrap().is_empty());
        assert_eq!(first_result_set("[]").unwrap().len(), 0);
        let rows = first_result_set("[{\"one\":1}]").unwrap();
        assert_eq!(rows.len(), 1);
        // A later statement's set is ignored: the caller asked for one result.
        let two = first_result_set("[{\"a\":1}]\n[{\"b\":2}]").unwrap();
        assert_eq!(two.len(), 1);
        assert!(two[0].get("a").is_some());
    }
}
