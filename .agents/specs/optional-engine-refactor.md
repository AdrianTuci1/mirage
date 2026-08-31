# Refactorizare motoare opționale: DuckDB și ONNX Runtime

Acest document definește refactorizarea daemonului Rust `src/daemon_next` astfel încât doar LanceDB + IPC + Modular Download Manager rămân în binarul de bază, iar DuckDB și ONNX Runtime devin module descărcabile la runtime în `<app-bundle>/downloads/`. Documentul este o extensie a ADR 012 și a specificației `modular-download-manager.md`.

## 1. Context și constrângeri

- Daemonul curent link-ează static `ort` (ONNX Runtime) și `duckdb` cu feature `bundled`, ducând la un binar de ~716 MB debug / ~284 MB release.
- Obiectivul ADR 012 este un installer mic; motoarele opționale se descarcă doar când utilizatorul le activează.
- Nu se modifică direct `src/daemon_next` în acest task; acest document este designul pe care implementarea îl va urma.
- Binarul de bază trebuie să pornească fără DuckDB/ONNX și să ofere fallback-uri sau erori structurate clare.

## 2. Strategie Cargo.toml

### 2.1 Alternative analizate

| Abordare | Pro | Contra | Recomandare |
|----------|-----|--------|-------------|
| **Feature flags** (`duckdb`, `onnx`) în `Cargo.toml` | Simplu, `cfg` compilează codul doar când e activ; teste ușoare fără motoare | Feature-urile sunt la compile-time, nu la runtime; build-ul oficial tot link-ează librăriile native dacă flagurile sunt active | Folosit ca **mecanism de dezvoltare/test** și pentru compatibilitatea cu `duckdb-rs`/`ort` |
| **Dynamic loading** via `libloading` + librării native descărcate | Binar mic; modulele se încarcă doar când sunt gata; respectă ADR 012 | Complexitate sporită pe fiecare platformă; trebuie gestionate semnăturile, căile și variabilele de mediu | **Abordare principală pentru MVP** |
| **dlopen static** / wrapper C | Cross-platform uniform | Necesită cod C suplimentar, dificil de întreținut | Nu pentru MVP |
| **Compilare condiționată doar cu feature-uri** | Build-uri mici dacă sunt dezactivate | Pierdem funcționalitatea când flagurile sunt off; nu e o soluție de descărcare la cerere | Complementară, nu suficientă |

### 2.2 Recomandare pentru MVP

Aplicăm o **strategie hibridă**:

1. **Cargo.toml** păstrează dependințele `duckdb` și `ort`, dar controlate prin feature flags implicite:

   ```toml
   [features]
   default = ["duckdb", "onnx"]
   duckdb = ["dep:duckdb"]
   onnx = ["dep:ort"]

   [dependencies]
   duckdb = { version = "1", features = ["bundled"], optional = true }
   ort = { version = "2.0.0-rc.13", optional = true }
   libloading = "0.8"
   ```

2. Pentru build-ul oficial de release, feature-urile `duckdb`/`onnx` rămân active, dar crate-urile sunt folosite cu **bundled disabled** (DuckDB) sau prin **variabile de medie** (ort), astfel încât binarul să nu includă librăriile native statice. Aceasta permite codul Rust să se compileze cu structurile de binding, iar librăriile native să fie încărcate dinamic din `downloads/`.

3. Când `duckdb`/`onnx` sunt dezactivate (de ex. build test rapid), codul de binding este înlocuit cu **stub-uri/mock** prin feature gate `#[cfg(feature = "duckdb")]` / `#[cfg(feature = "onnx")]`.

4. `libloading` rămâne obligatoriu (non-optional) pentru a încărca librăriile native la runtime.

### 2.3 Implicații pentru CI/test

- `cargo test --no-default-features` rulează teste fără DuckDB/ONNX, folosind stub-uri.
- `cargo test --all-features` rulează cu bindingurile reale; se folosesc module stub descărcate local pentru a nu accesa CDN în teste.

## 3. Schimbări în `DaemonConfig`

Structura `DaemonConfig` (actuală în `src/daemon_next/src/config.rs`) primește un câmp `downloads_dir` și extinde `ModulesConfig` cu `sql_generator`:

```yaml
# daemon.yaml
data_dir: <app-bundle>/data
models_dir: <app-bundle>/models
downloads_dir: <app-bundle>/downloads
socket_path: <app-bundle>/mirage.sock
log_level: info
modules:
  vector: true        # LanceDB, mereu activ
  text: true          # ONNX text embeddings, descărcabil
  tabular: false      # DuckDB analytics, descărcabil
  sql_generator: false # SLM SQL generator, descărcabil
  audio: false
  vision: false
```

### 3.1 Câmpuri Rust propuse

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct ModulesConfig {
    pub vector: bool,          // mereu true
    pub text: bool,            // necesită onnx_runtime + text_embedding_model
    pub tabular: bool,         // necesită duckdb
    pub sql_generator: bool,   // necesită onnx_runtime + duckdb + slm_nl_router
    pub audio: bool,
    pub vision: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DaemonConfig {
    pub data_dir: PathBuf,
    pub models_dir: PathBuf,
    pub downloads_dir: PathBuf,
    #[cfg(unix)]
    pub socket_path: PathBuf,
    #[cfg(windows)]
    pub pipe_name: String,
    pub log_level: String,
    pub modules: ModulesConfig,
    pub download_manager: DownloadManagerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DownloadManagerConfig {
    pub catalog_url: String,
    pub signature_key_fingerprint: String,
    pub auto_download_dependencies: bool,
}
```

### 3.2 Semantica flagurilor `modules`

- `vector`: întotdeauna `true`; reprezintă capabilitatea de bază (LanceDB + căutare vectorială).
- `text`: `true` dacă utilizatorul a optat pentru embeddings text; daemonul va încerca să descarce/încarce `onnx_runtime` și `text_embedding_model`.
- `tabular`: `true` dacă utilizatorul a optat pentru analytics SQL; daemonul va încerca să descarce/încarce `duckdb`.
- `sql_generator`: `true` dacă utilizatorul a optat pentru asistentul în limbaj natural (routing + SQL + sumarizare); necesită `onnx_runtime`, `duckdb` și `slm_nl_router`.

Aceste flaguri sunt preferințe declarative; starea efectivă se determină din `ModuleManager::status(module_id)`.

## 4. Secvența de startup refactorizată

```
1. Parse CLI args + load daemon.yaml
2. Init logging
3. Ensure dirs: data/, models/, downloads/
4. Start ModuleManager (catalog local, state.json, evenimente IPC)
5. Open LanceDbStore (mereu disponibil)
6. Start IpcServer cu:
   - Arc<LanceDbStore> (obligatoriu)
   - Option<Arc<Mutex<dyn Embedder>>> (None inițial)
   - Option<Arc<Mutex<Analytics>>> (None inițial)
   - Arc<ModuleManager>
7. Într-un task separat, ModuleManager evaluează config.modules:
   - dacă text=true și modulele sunt deja descărcate, încarcă ONNX Runtime + model și notifică daemonul
   - dacă tabular=true și duckdb este descărcat, încarcă DuckDB engine și notifică daemonul
8. Daemonul răspunde la IPC chiar dacă modulele opționale nu sunt gata
```

### 4.1 Activare la cerere

Când un client apelează `download_module("duckdb")`:

1. `ModuleManager` verifică catalogul semnat.
2. Descarcă arhiva în `downloads/duckdb/<version>/`.
3. Verifică SHA-256, extrage, setează stare `ready`.
4. Emite `ModuleEvent { module_id: "duckdb", state: "ready" }`.
5. Daemonul primește evenimentul și apelează `load_duckdb_engine()`.
6. `IpcServer` își actualizează `Option<Arc<Mutex<Analytics>>>` din `None` în `Some(...)`.

## 5. Verificarea disponibilității modulelor înainte de operații

### 5.1 Mapping metode IPC → module necesare

| Metodă IPC | Module necesare | Comportament dacă lipsește |
|------------|-----------------|--------------------------|
| `search(query)` | `onnx_runtime` + `text_embedding_model` (sau fallback) | Dacă ONNX e absent și fallback e activ, folosește fallback; altfel eroare structurată |
| `embed(text)` | `onnx_runtime` + `text_embedding_model` (sau fallback) | Idem |
| `query(sql)` | `duckdb` | Eroare structurată; propune descărcarea |
| `ask(question)` | `onnx_runtime` + `duckdb` + `slm_nl_router` | Eroare structurată cu prima dependență lipsă; răspuns NL sau rezultate semantice |
| `index(records)` | `vector` (LanceDB) obligatoriu | Funcționează mereu; vectorii pot fi generați cu fallback dacă ONNX e absent |
| `status()` | niciunul | Raportează module active/descărcate |

### 5.2 Ordinea verificării

Fiecare handler IPC va apela un helper comun:

```rust
fn ensure_modules(mm: &ModuleManager, required: &[&str]) -> Result<(), ModuleMissingError> {
    for module_id in required {
        if mm.status(module_id).state != ModuleState::Ready {
            return Err(ModuleMissingError::from_manager(module_id, mm));
        }
    }
    Ok(())
}
```

Pentru `ask`, se verifică în ordinea: `slm_nl_router`, `onnx_runtime`, `duckdb`. Prima lipsă determină eroarea returnată.

## 6. Eroare structurată pentru modul lipsă

Toate metodele IPC care necesită un modul opțional vor returna un răspuns JSON-RPC cu cod dedicat și câmp `data` structurat.

### 6.1 Coduri de eroare noi

Adăugăm în `src/daemon_next/src/ipc/protocol.rs`:

```rust
pub const ERROR_MODULE_MISSING: i32 = -32001;
pub const ERROR_MODULE_LOAD_FAILED: i32 = -32002;
pub const ERROR_MODULE_DOWNLOAD_IN_PROGRESS: i32 = -32003;
```

### 6.2 Formatul erorii

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32001,
    "message": "Module not installed",
    "data": {
      "error_kind": "module_missing",
      "module_id": "duckdb",
      "required_version": "1.1.3",
      "module_name": "DuckDB Analytics Engine",
      "module_size": 16777216,
      "dependencies": [],
      "download_url_hint": "https://cdn.mirage.ai/catalog.json",
      "reason": "DuckDB is required for SQL analytics but is not downloaded."
    }
  },
  "id": 7
}
```

### 6.3 Structura Rust propusă

```rust
#[derive(Debug, Clone, Serialize)]
pub enum ModuleErrorKind {
    ModuleMissing,
    ModuleLoadFailed,
    ModuleDownloadInProgress,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModuleErrorData {
    pub error_kind: ModuleErrorKind,
    pub module_id: String,
    pub required_version: Option<String>,
    pub module_name: Option<String>,
    pub module_size: Option<u64>,
    pub dependencies: Vec<String>,
    pub download_url_hint: Option<String>,
    pub reason: String,
}
```

Helperul de handler va converti `ModuleMissingError` în `JsonRpcError` cu `code = ERROR_MODULE_MISSING` și `data` serializat.

## 7. Refactorizare `IpcServer::new()`

Semnătura curentă:

```rust
pub fn new(
    store: Arc<LanceDbStore>,
    embedder: Arc<dyn Embedder>,
    analytics: Arc<Analytics>,
) -> Self
```

Devine:

```rust
pub fn new(
    store: Arc<LanceDbStore>,
    embedder: Arc<Mutex<Option<Arc<dyn Embedder>>>>,
    analytics: Arc<Mutex<Option<Arc<Analytics>>>>,
    module_manager: Arc<ModuleManager>,
) -> Self
```

### 7.1 Alternative discutate

- `Arc<Mutex<Option<Arc<...>>>>` este cea mai simplă variantă pentru MVP: permite înlocuirea atomică la runtime (când un modul devine ready) și este compatibilă cu `spawn_blocking`.
- Un `tokio::sync::RwLock<Option<...>>` ar permite cititori multipli, dar operatorile de embed/query sunt oricum blocate pe mutexul engine-ului.
- Un `Option` pur, fără `Mutex`, nu permite actualizarea după construire.

### 7.2 Actualizarea modulelor la runtime

`ModuleManager` emite evenimente printr-un canal `tokio::sync::mpsc`:

```rust
pub enum ModuleEvent {
    Ready { module_id: String, version: String },
    Error { module_id: String, error: String },
    Removed { module_id: String },
}
```

Un task în `main()` ascultă evenimentele și, la `Ready`, încarcă engine-ul corespunzător:

```rust
match event {
    ModuleEvent::Ready { module_id: "duckdb", version } => {
        match load_duckdb_engine(&config, version) {
            Ok(engine) => {
                let mut guard = analytics.lock().unwrap();
                *guard = Some(Arc::new(engine));
            }
            Err(e) => { /* log + set module state error */ }
        }
    }
    ModuleEvent::Ready { module_id: "onnx_runtime", version } => {
        // reîncarcă OnnxEmbedder dacă text=true și modelul text este ready
    }
    // ...
}
```

### 7.3 Status handler actualizat

`status()` va raporta starea reală a modulelor, nu hardcodat `true`:

```json
{
  "status": "ok",
  "version": "0.1.0",
  "vector_count": 0,
  "modules": {
    "vector": true,
    "text": false,
    "tabular": false,
    "sql_generator": false,
    "audio": false,
    "vision": false
  },
  "module_states": {
    "duckdb": "missing",
    "onnx_runtime": "missing",
    "text_embedding_model": "missing"
  }
}
```

## 8. Fallback embedder când ONNX lipsește

Există deja `FallbackEmbedder` în `src/daemon_next/src/embeddings.rs`. Logica de construire devine:

```rust
pub fn create_embedder(
    config: &DaemonConfig,
    module_manager: &ModuleManager,
) -> Arc<dyn Embedder> {
    let onnx_ready = module_manager.status("onnx_runtime").state == ModuleState::Ready
        && module_manager.status("text_embedding_model").state == ModuleState::Ready;

    if onnx_ready {
        if let Some(model_path) = find_onnx_model(&config.models_dir) {
            match OnnxEmbedder::new(model_path) {
                Ok(onnx) => return Arc::new(onnx),
                Err(err) => {
                    tracing::warn!("Failed to load ONNX model: {err}. Using fallback embedder.");
                }
            }
        }
    }

    Arc::new(FallbackEmbedder::new(DEFAULT_EMBEDDING_DIM))
}
```

### 8.1 Comportament produs vs. test

- **Producție**: fallback determinist este acceptat doar ca strat de siguranță; clientul primește avertisment în `status()` că embeddings-ul este aproximativ. Recomandarea este să descarce `text_embedding_model`.
- **Teste**: fallback este activ implicit atunci când testele rulează cu `--no-default-features`, permițând pipeline-ul fără descărcări.

### 8.2 Consistență LanceDB

Vectorii produși de fallback au aceeași dimensiune (384) ca cei ONNX, deci schema LanceDB rămâne compatibilă. Căutarea cosinus funcționează corect, chiar dacă semantica este deterministă.

## 9. Încărcarea dinamică a DuckDB

> **Neaplicabil (ADR 014).** Secțiunea propune `libloading` + `libduckdb.dylib` descărcat. Crate-ul `duckdb` rezolvă librăria nativă la
> link-time (rustc scrie `LC_LOAD_DYLIB` în executabil), deci variabilele de mediu setate la runtime nu schimbă nimic, iar §9.4 recunoaște
> aceeași limitare pe macOS. Implementarea aleasă este binarul DuckDB CLI descărcat ca modul `runtime` și rulat ca proces copil: vezi
> ADR 014 și `src/daemon_next/src/analytics.rs`. Textul de mai jos rămâne ca istoric al analizei.
>
> **ONNX, spre deosebire de DuckDB, chiar folosește încărcarea dinamică:** crate-ul `ort` e compilat cu feature-ul `load-dynamic`
> (echivalentul `libloading` din §9), modulul `onnx_runtime` descarcă biblioteca partajată, iar daemonul setează `ORT_DYLIB_PATH` la ea —
> deci partea de încărcare dinamică a acestui doc se aplică și s-a implementat pentru ONNX.

### 9.1 Layout așteptat

```
<app-bundle>/downloads/duckdb/<version>/
  lib/
    libduckdb.dylib   # macOS
    libduckdb.so      # Linux
    duckdb.dll        # Windows (împreună cu dependențe .dll)
```

### 9.2 Strategie cu `duckdb-rs` și `bundled` disabled

Crate-ul `duckdb` Rust oferă două moduri:

1. **bundled** (default): compilează/linkează static DuckDB C++ în binar. Nu este acceptabil pentru obiectivul de binar mic.
2. **unbundled**: caută librăria nativă la link-time sau runtime.

Pentru MVP propunem:

- Deactivăm feature-ul `bundled` în `Cargo.toml`:

  ```toml
  duckdb = { version = "1", default-features = false, optional = true }
  ```

- Setăm variabila de mediu `DUCKDB_LIB_DIR` la directorul `lib/` al versiunii descărcate **înainte** de a inițializa o conexiune.

- Folosim `libloading` pentru a încărca explicit `libduckdb` înainte de primul `Connection::open`, astfel încât simbolurile să fie rezolvate dinamic.

### 9.3 Cod propus

```rust
#[cfg(feature = "duckdb")]
use duckdb::Connection;
use libloading::Library;
use std::path::PathBuf;

fn library_filename(base: &str) -> PathBuf {
    #[cfg(target_os = "macos")]
    { PathBuf::from(format!("lib{base}.dylib")) }
    #[cfg(target_os = "linux")]
    { PathBuf::from(format!("lib{base}.so")) }
    #[cfg(target_os = "windows")]
    { PathBuf::from(format!("{base}.dll")) }
}

pub fn load_duckdb_lib(downloads_dir: &Path, version: &str) -> Result<Library, ModuleError> {
    let lib_path = downloads_dir
        .join("duckdb")
        .join(version)
        .join("lib")
        .join(library_filename("duckdb"));

    if !lib_path.exists() {
        return Err(ModuleError::missing("duckdb", "library not found"));
    }

    unsafe {
        Library::new(&lib_path)
            .map_err(|e| ModuleError::load_failed("duckdb", &format!("{e}")))
    }
}

pub fn set_duckdb_env(downloads_dir: &Path, version: &str) {
    let lib_dir = downloads_dir.join("duckdb").join(version).join("lib");
    std::env::set_var("DUCKDB_LIB_DIR", &lib_dir);
    #[cfg(target_os = "macos")]
    {
        let current = std::env::var("DYLD_LIBRARY_PATH").unwrap_or_default();
        std::env::set_var("DYLD_LIBRARY_PATH", format!("{}:{}", lib_dir.display(), current));
    }
    #[cfg(target_os = "linux")]
    {
        let current = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
        std::env::set_var("LD_LIBRARY_PATH", format!("{}:{}", lib_dir.display(), current));
    }
    #[cfg(target_os = "windows")]
    {
        let current = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{};{}", lib_dir.display(), current));
    }
}
```

### 9.4 Platform-specific notes

- **macOS**: `DYLD_LIBRARY_PATH` este adesea ignorat pentru procese cu hardening (SIP) sau aplicații semnate. Soluție robustă: copierea/legarea simbolică a `.dylib` într-un director `@rpath` al binarului sau folosirea `install_name_tool` la pachetare. Pentru MVP, semnalăm această limitare și recomandăm ca librăriile descărcate să aibă `@loader_path`/`@rpath` configurat corect.
- **Windows**: `LoadLibraryW` caută DLL în directorul executabilului, în `PATH` și în System32. Setarea `PATH` include directorul `lib/` descărcat. Dependențele VC++ redistribuibile trebuie incluse în pachetul DuckDB.
- **Linux**: `dlopen` cu cale absolută funcționează fiabil. Dependențele `libduckdb.so` (de ex. `libstdc++`) trebuie să fie prezente pe sistem sau incluse în pachet.

### 9.5 Construirea `Analytics` lazy

`Analytics::open` va primi opțional calea către librăria nativă și va încărca mai întâi librăria, apoi va deschide conexiunea:

```rust
impl Analytics {
    pub fn open(config: &DaemonConfig, version: &str) -> Result<Self> {
        let _lib = load_duckdb_lib(&config.downloads_dir, version)?;
        set_duckdb_env(&config.downloads_dir, version);
        let db_path = config.data_dir.join("analytics.duckdb");
        // ... restul codului existent
    }
}
```

Variabila `_lib` trebuie reținută în structura `Analytics` pentru a preveni descărcarea (`Library` are lifetime-ul procesului).

## 10. Încărcarea dinamică a ONNX Runtime pentru `ort`

### 10.1 Layout așteptat

```
<app-bundle>/downloads/onnx_runtime/<version>/
  lib/
    libonnxruntime.dylib  # macOS
    libonnxruntime.so     # Linux
    onnxruntime.dll       # Windows
```

### 10.2 Strategie cu crate-ul `ort`

Crate-ul `ort` 2.x caută librăria nativă în mai multe locuri:

1. Variabila de mediu `ORT_LIB_LOCATION` (directorul ce conține `lib/`).
2. Variabila `ORT_DYLIB_PATH` (cale directă către librărie).
3. Căi predefinite în sistem.

Pentru MVP propunem:

- Build-ul oficial setează `ort` fără feature-ul de download implicit (`download-binaries` disabled) pentru a evita descărcarea la compile-time.
- La runtime, înainte de prima sesiune ONNX, apelăm `load_onnx_runtime_lib()` cu `libloading` și setăm `ORT_DYLIB_PATH`/`ORT_LIB_LOCATION` la calea descărcată.

### 10.3 Cod propus

```rust
pub fn load_onnx_runtime_lib(downloads_dir: &Path, version: &str) -> Result<Library, ModuleError> {
    let lib_path = downloads_dir
        .join("onnx_runtime")
        .join(version)
        .join("lib")
        .join(library_filename("onnxruntime"));

    if !lib_path.exists() {
        return Err(ModuleError::missing("onnx_runtime", "library not found"));
    }

    unsafe {
        Library::new(&lib_path)
            .map_err(|e| ModuleError::load_failed("onnx_runtime", &format!("{e}")))
    }
}

pub fn set_onnx_env(downloads_dir: &Path, version: &str) {
    let lib_dir = downloads_dir.join("onnx_runtime").join(version).join("lib");
    let lib_file = lib_dir.join(library_filename("onnxruntime"));

    std::env::set_var("ORT_LIB_LOCATION", &lib_dir);
    std::env::set_var("ORT_DYLIB_PATH", &lib_file);

    #[cfg(target_os = "macos")]
    {
        let current = std::env::var("DYLD_LIBRARY_PATH").unwrap_or_default();
        std::env::set_var("DYLD_LIBRARY_PATH", format!("{}:{}", lib_dir.display(), current));
    }
    #[cfg(target_os = "linux")]
    {
        let current = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
        std::env::set_var("LD_LIBRARY_PATH", format!("{}:{}", lib_dir.display(), current));
    }
    #[cfg(target_os = "windows")]
    {
        let current = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{};{}", lib_dir.display(), current));
    }
}
```

### 10.4 Inițializare `OnnxEmbedder`

```rust
impl OnnxEmbedder {
    pub fn open(config: &DaemonConfig, runtime_version: &str, model_id: &str) -> Result<Self> {
        let _lib = load_onnx_runtime_lib(&config.downloads_dir, runtime_version)?;
        set_onnx_env(&config.downloads_dir, runtime_version);

        let model_path = config.models_dir.join(model_id).join("model.onnx");
        // ... restul constructorului existent
    }
}
```

`Library` va fi reținută în `OnnxEmbedder` pentru a menține librăria încărcată.

### 10.5 Alternative pentru `ort`

Dacă `ort` nu respectă variabilele de mediu în mod robust, se poate adăuga un wrapper care:

1. Folosește `ort` cu API direct.
2. Încarcă librăria nativă cu `libloading` într-un block `unsafe` separat.
3. Setează căile înainte de a apela orice funcție `ort`.

În cazuri extreme, se poate evalua migrarea la `onnxruntime-rs` sau la un binding custom minim, dar aceasta depășește MVP-ul.

## 11. Siguranța în context de thread-uri

- Modulele sunt **descărcate o singură dată** și devin `ready`. Nu există unload în MVP; ștergerea unui modul necesită restart.
- `Arc<Mutex<Option<Arc<Analytics>>>>` și similar pentru embedder permit unui singur task să înlocuiască valoarea atunci când modulul devine ready. Cititorii (handlerii IPC) iau o clonă `Arc` sub lock și o eliberează înainte de a executa operația blocantă.
- Operațiile DuckDB și ONNX rulează în `spawn_blocking` pentru a nu bloca runtime-ul async. Fiecare engine este `Send + Sync` și protejat de propriul `Mutex` intern.
- `ModuleManager` este `Sync` și metodele de citire a stării nu fac mutate; evenimentele vin prin `mpsc` serializate.

## 12. Testing strategy

### 12.1 Unit tests

- Teste pentru parsarea `ModulesConfig` și serializarea `DaemonConfig` (extind testele existente din `config.rs`).
- Teste pentru `ModuleError` → `JsonRpcError` mapping.
- Teste pentru fallback embedder (deja existente; se extind cu aserțiuni pe dimensiune 384).

### 12.2 Integration tests cu module stub

În `src/daemon_next/tests/` se adaugă:

- `tests/module_missing_error.rs`: porneste daemonul fără module descărcate, trimite `query` și verifică codul `-32001` cu `module_id: "duckdb"`.
- `tests/download_and_load_stub.rs`: creează un catalog local cu URL-uri `file://` către arhive stub, apelează `download_module`, așteaptă `ready`, apoi `query` returnează rezultate.
- `tests/fallback_embed.rs`: rulează fără ONNX și verifică că `embed` și `search` funcționează cu fallback.

### 12.3 Stub modules

Se creează în `src/daemon_next/tests/fixtures/`:

- Stub `libduckdb.dylib`/`.so`/`.dll`: o librărie dinamică minimă care expune simbolurile necesare doar pentru teste. De preferat, se folosește o versiune reală DuckDB mică pre-compilată pentru CI.
- Stub `libonnxruntime` + model ONNX trivial (de ex. un model identity) pentru testarea `OnnxEmbedder`.

### 12.4 CI

- `cargo test --no-default-features`: fără DuckDB/ONNX, folosește doar fallback.
- `cargo test --all-features`: cu bindinguri reale și stub-uri locale.
- Niciun test nu accesează rețeaua în CI; toate descărcările folosesc `file://` sau mock HTTP server.

## 13. Plan de implementare (ordinea fișierelor)

1. `src/daemon_next/Cargo.toml`: feature flags + `libloading`, `bundled` disabled.
2. `src/daemon_next/src/config.rs`: adaugă `downloads_dir`, `sql_generator`, `DownloadManagerConfig`.
3. `src/daemon_next/src/modules/mod.rs`, `module_manager.rs`, `module_state.rs`: manager de module și mașina de stări.
4. `src/daemon_next/src/modules/duckdb_engine.rs`: încărcare dinamică + wrapper `Analytics`.
5. `src/daemon_next/src/modules/onnx_engine.rs`: încărcare runtime + wrapper `OnnxEmbedder`.
6. `src/daemon_next/src/ipc/protocol.rs`: noi coduri de eroare și `ModuleErrorData`.
7. `src/daemon_next/src/ipc/server.rs`: `IpcServer::new()` cu `Option` pentru embedder/analytics, verificări de module.
8. `src/daemon_next/src/main.rs`: startup refactorizat, task de ascultare evenimente module.
9. `src/daemon_next/tests/module_*.rs`: teste cu stub-uri.

## 14. Referințe

- ADR 012: `.agents/decisions/adr/012-modular-download-manager.md`
- Modular Download Manager spec: `.agents/specs/modular-download-manager.md`
- Module manifest schema: `.agents/specs/module-manifest-schema.json`
- Technical spec: `.agents/specs/technical-spec.md` §3.1.2, §3.1.4
- Implementație curentă: `src/daemon_next/src/embeddings.rs`, `src/daemon_next/src/analytics.rs`, `src/daemon_next/src/ipc/server.rs`, `src/daemon_next/src/config.rs`
