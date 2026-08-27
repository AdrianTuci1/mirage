# Modular Download Manager — Design Specification

## 1. Scop

Această specificație definește sistemul modular de descărcări pentru daemonul Rust Mirage. Scopul este ca binarul de bază să rămână mic, iar modulele opționale (DuckDB, ONNX Runtime, modele ML) să fie descărcate la cerere în interiorul bundle-ului aplicației. Toate datele rămân în `<app-bundle>`; la dezinstalare tot conținutul este șters.

## 2. Manifestul unui modul

Fiecare modul este descris de un manifest JSON care conține metadatele necesare pentru descărcare, verificare și activare.

### 2.1 Câmpuri obligatorii

| Câmp | Tip | Descriere |
|------|-----|-----------|
| `id` | string | Identificator unic, lowercase, underscore: `duckdb`, `onnx_runtime`. |
| `name` | string | Nume prietenos pentru UI. |
| `version` | string | Versiune semantică (SemVer): `1.1.3`. |
| `description` | string | Scurtă descriere afișată în wizard și Settings. |
| `kind` | string | Tip modul: `runtime`, `library`, `model`. |
| `license` | string | Identificator SPDX sau URL către licență: `MIT`, `https://...`. |
| `platforms` | object | Mapare per platformă: `macos_aarch64`, `macos_x86_64`, `windows_x86_64`, `linux_x86_64`, `linux_aarch64`. |
| `dependencies` | array | Lista de `id`-uri ale modulelor care trebuie să fie ready înainte de activare. |
| `is_optional` | boolean | `true` pentru module care nu vin în binarul de bază. |

### 2.2 Câmpuri per platformă

Fiecare intrare din `platforms` conține:

| Câmp | Tip | Descriere |
|------|-----|-----------|
| `url` | string (URL) | URL HTTPS direct către arhivă. |
| `size` | integer | Dimensiunea în bytes a arhivei. |
| `checksum` | string | SHA-256 hex al arhivei. |
| `archive_format` | string | `tar.gz`, `zip`, `raw`. |
| `files` | array | Lista fișierelor așteptate după extracție (căi relative, checksum per fișier). |

Câmpul `files` este un array de obiecte:

```json
{
  "relative_path": "lib/libduckdb.dylib",
  "sha256": "abc123...",
  "executable": false,
  "required": true
}
```

### 2.3 Exemplu minim

```json
{
  "id": "duckdb",
  "name": "DuckDB Analytics Engine",
  "version": "1.1.3",
  "description": "OLAP SQL engine for tabular analytics.",
  "kind": "runtime",
  "license": "MIT",
  "is_optional": true,
  "dependencies": [],
  "platforms": {
    "macos_aarch64": {
      "url": "https://cdn.mirage.ai/modules/duckdb/1.1.3/duckdb-macos-aarch64.tar.gz",
      "size": 16777216,
      "checksum": "c3ab8ff137...",
      "archive_format": "tar.gz",
      "files": [
        {
          "relative_path": "lib/libduckdb.dylib",
          "sha256": "a1b2c3...",
          "executable": true,
          "required": true
        }
      ]
    }
  }
}
```

## 3. Exemple de module

### 3.1 `duckdb`

```json
{
  "id": "duckdb",
  "name": "DuckDB Analytics Engine",
  "version": "1.1.3",
  "description": "OLAP SQL engine for tabular analytics and natural-language SQL generation.",
  "kind": "runtime",
  "license": "MIT",
  "is_optional": true,
  "dependencies": [],
  "platforms": {
    "macos_aarch64": { "url": ".../duckdb-1.1.3-macos-aarch64.tar.gz", "size": 16777216, "checksum": "...", "archive_format": "tar.gz", "files": [{"relative_path":"lib/libduckdb.dylib","sha256":"...","executable":true,"required":true}] },
    "macos_x86_64": { "url": ".../duckdb-1.1.3-macos-x86_64.tar.gz", "size": 18874368, "checksum": "...", "archive_format": "tar.gz", "files": [{"relative_path":"lib/libduckdb.dylib","sha256":"...","executable":true,"required":true}] },
    "windows_x86_64": { "url": ".../duckdb-1.1.3-windows-x86_64.zip", "size": 20971520, "checksum": "...", "archive_format": "zip", "files": [{"relative_path":"bin/duckdb.dll","sha256":"...","executable":true,"required":true}] },
    "linux_x86_64": { "url": ".../duckdb-1.1.3-linux-x86_64.tar.gz", "size": 15728640, "checksum": "...", "archive_format": "tar.gz", "files": [{"relative_path":"lib/libduckdb.so","sha256":"...","executable":true,"required":true}] },
    "linux_aarch64": { "url": ".../duckdb-1.1.3-linux-aarch64.tar.gz", "size": 14680064, "checksum": "...", "archive_format": "tar.gz", "files": [{"relative_path":"lib/libduckdb.so","sha256":"...","executable":true,"required":true}] }
  }
}
```

### 3.2 `onnx_runtime`

```json
{
  "id": "onnx_runtime",
  "name": "ONNX Runtime",
  "version": "1.19.0",
  "description": "Cross-platform inference runtime for ONNX models.",
  "kind": "runtime",
  "license": "MIT",
  "is_optional": true,
  "dependencies": [],
  "platforms": {
    "macos_aarch64": { "url": ".../onnxruntime-1.19.0-macos-aarch64.tar.gz", "size": 10485760, "checksum": "...", "archive_format": "tar.gz", "files": [{"relative_path":"lib/libonnxruntime.dylib","sha256":"...","executable":true,"required":true}] },
    "macos_x86_64": { "url": ".../onnxruntime-1.19.0-macos-x86_64.tar.gz", "size": 11534336, "checksum": "...", "archive_format": "tar.gz", "files": [{"relative_path":"lib/libonnxruntime.dylib","sha256":"...","executable":true,"required":true}] },
    "windows_x86_64": { "url": ".../onnxruntime-1.19.0-windows-x86_64.zip", "size": 12582912, "checksum": "...", "archive_format": "zip", "files": [{"relative_path":"lib/onnxruntime.dll","sha256":"...","executable":true,"required":true}] },
    "linux_x86_64": { "url": ".../onnxruntime-1.19.0-linux-x86_64.tar.gz", "size": 9437184, "checksum": "...", "archive_format": "tar.gz", "files": [{"relative_path":"lib/libonnxruntime.so","sha256":"...","executable":true,"required":true}] },
    "linux_aarch64": { "url": ".../onnxruntime-1.19.0-linux-aarch64.tar.gz", "size": 9437184, "checksum": "...", "archive_format": "tar.gz", "files": [{"relative_path":"lib/libonnxruntime.so","sha256":"...","executable":true,"required":true}] }
  }
}
```

### 3.3 `text_embedding_model`

```json
{
  "id": "text_embedding_model",
  "name": "Text Embedding Model",
  "version": "1.0.0",
  "description": "ONNX model for 384-dimensional text embeddings (all-MiniLM-L6-v2 style).",
  "kind": "model",
  "license": "Apache-2.0",
  "is_optional": true,
  "dependencies": ["onnx_runtime"],
  "platforms": {
    "universal": {
      "url": ".../text-embedding-model-1.0.0-universal.tar.gz",
      "size": 83886080,
      "checksum": "...",
      "archive_format": "tar.gz",
      "files": [
        {"relative_path":"model.onnx","sha256":"...","executable":false,"required":true},
        {"relative_path":"tokenizer.json","sha256":"...","executable":false,"required":true}
      ]
    }
  }
}
```

### 3.4 `slm_nl_router`

```json
{
  "id": "slm_nl_router",
  "name": "SLM Natural Language Router",
  "version": "1.0.0",
  "description": "Multilingual ONNX model that routes user questions to semantic search or SQL, generates DuckDB SQL, and summarizes results in natural language.",
  "kind": "model",
  "license": "Apache-2.0",
  "is_optional": true,
  "dependencies": ["onnx_runtime", "duckdb"],
  "platforms": {
    "universal": {
      "url": ".../slm-nl-router-1.0.0-universal.tar.gz",
      "size": 150000000,
      "checksum": "...",
      "archive_format": "tar.gz",
      "files": [
        {"relative_path":"model.onnx","sha256":"...","executable":false,"required":true},
        {"relative_path":"tokenizer.json","sha256":"...","executable":false,"required":true}
      ]
    }
  }
}
```

### 3.5 `vision_model`

```json
{
  "id": "vision_model",
  "name": "Vision Embedding Model",
  "version": "1.0.0",
  "description": "ONNX CLIP-style model for image embeddings.",
  "kind": "model",
  "license": "Apache-2.0",
  "is_optional": true,
  "dependencies": ["onnx_runtime"],
  "platforms": {
    "universal": {
      "url": ".../vision-model-1.0.0-universal.tar.gz",
      "size": 209715200,
      "checksum": "...",
      "archive_format": "tar.gz",
      "files": [
        {"relative_path":"model.onnx","sha256":"...","executable":false,"required":true},
        {"relative_path":"preprocessor.json","sha256":"...","executable":false,"required":true}
      ]
    }
  }
}
```

## 4. Catalogul de module

Locația în repository: `assets/modules/catalog.json`.

La runtime, daemonul îl descarcă periodic de la un URL well-known (de ex. `https://cdn.mirage.ai/catalog.json`) și îl salvează în `<app-bundle>/downloads/catalog.json`. Catalogul este semnat digital și include metadate de versiune.

```json
{
  "schema_version": "1.0.0",
  "catalog_version": "2026.08.27-1",
  "minimum_daemon_version": "0.2.0",
  "signature": {
    "algorithm": "ed25519",
    "public_key_fingerprint": "a1b2c3...",
    "signature": "base64signature"
  },
  "modules": [
    { /* duckdb manifest */ },
    { /* onnx_runtime manifest */ },
    { /* text_embedding_model manifest */ },
    { /* slm_nl_router manifest */ },
    { /* vision_model manifest */ }
  ]
}
```

### 4.1 Semnătura

- Corpul JSON al catalogului este serializat canonical (chei sortate, fără whitespace).
- Semnătura este calculată cu cheia privată Mirage și verificată cu public key embedded în daemon.
- Dacă semnătura nu este validă, catalogul este respins și daemonul păstrează catalogul anterior (dacă există).

### 4.2 Versioning

- `schema_version` este SemVer; daemonul acceptă doar versiuni de catalog compatibile.
- `catalog_version` este un identificator unic incremental.
- `minimum_daemon_version` permite deprecarea modulelor pentru daemoni vechi.

## 5. Responsabilitățile Download Manager

Managerul este un modul Rust în daemon (`src/modules/download_manager.rs`) care rulează pe un task async separat.

### 5.1 Funcții principale

1. **Descărcare HTTPS cu resume**
   - Folosește `reqwest` cu `Range` headers.
   - Fișierele parțiale sunt salvate cu extensia `.part`.
   - Dacă un `.part` există și serverul acceptă resume, descărcarea continuă de la offset-ul existent.
   - Dacă checksum-ul parțial nu corespunde la reîncepere, fișierul este șters și descărcat de la capăt.

2. **Verificare checksum**
   - După descărcare, se calculează SHA-256 al arhivei și se compară cu manifestul.
   - Dacă nu corespunde, fișierul este șters și starea devine `error`.

3. **Extracție atomică**
   - Arhiva este extrasă într-un director temporar `<module-id>-<version>.tmp`.
   - După succes, directorul temporar este redenumit atomic în `<module-id>/<version>/`.
   - Pe Windows, redenumirea atomică este realizată prin `MoveFileEx` cu `MOVEFILE_REPLACE_EXISTING`.

4. **Rezolvarea dependențelor**
   - Înainte de activare, se verifică că toate modulele din `dependencies` sunt `ready`.
   - Dacă o dependență lipsește, managerul o descarcă automat înaintea modulului cerut (cu confirmare UI doar pentru modulul final, nu pentru dependențe ascunse, dar dimensiunea totală este comunicată).

5. **Progress reporting**
   - Canal intern de evenimente: `ModuleEvent { module_id, version, state, bytes_downloaded, bytes_total, error: Option<String> }`.
   - Evenimentele sunt propagate prin IPC către GUI/CLI.

6. **Gestiunea spațiului**
   - Înainte de descărcare se verifică spațiul liber minim (de ex. 2x dimensiunea arhivei).
   - Dacă nu există suficient spațiu, se returnează eroare `insufficient_disk_space`.

## 6. Layout de stocare

Toate căile sunt relative la `<app-bundle>`.

```
<app-bundle>/
  daemon.yaml
  data/
  downloads/
    catalog.json
    catalog.json.sig
    duckdb/
      1.1.3/
        lib/
          libduckdb.dylib
    onnx_runtime/
      1.19.0/
        lib/
          libonnxruntime.dylib
  models/
    text_embedding_model/
      1.0.0/
        model.onnx
        tokenizer.json
    slm_nl_router/
      1.0.0/
        model.onnx
        vocab.json
    vision_model/
      1.0.0/
        model.onnx
        preprocessor.json
```

Note:

- `downloads/` conține runtime-uri și librării native.
- `models/` conține modele ONNX și fișierele asociate.
- Un modul poate fi mutat în `models/` dacă `kind == "model"`, chiar dacă în manifest este generic.
- Directorul `<app-bundle>` este determinat la pornire ca fiind directorul părinte al executabilului daemonului.

## 7. Mașina de stări a modulului

```
missing -> downloading -> verifying -> ready -> removing
   |           |            |          |          |
   |           v            v          v          v
   +--------------------> error <------------------+
```

| Stare | Semnificație |
|-------|--------------|
| `missing` | Modulul nu este instalat. |
| `queued` | Așteaptă în coada de descărcări. |
| `downloading` | Se descarcă arhiva. |
| `paused` | Descărcarea a fost oprită temporar (user sau rețea). |
| `verifying` | Verificare SHA-256 și extragere atomică. |
| `ready` | Modulul este disponibil și poate fi încărcat. |
| `error` | Eroare de descărcare, verificare sau activare. |
| `removing` | Se șterge directorul modulului. |

Tranzițiile sunt persistate într-un fișier JSON în `<app-bundle>/downloads/state.json`.

## 8. Metode IPC

Extensiile se adaugă la protocolul JSON-RPC 2.0 existent. Parametrii `id` și `jsonrpc` sunt omisi pentru claritate.

### 8.1 `download_module`

Request:

```json
{
  "method": "download_module",
  "params": { "module_id": "duckdb", "force": false },
  "id": 10
}
```

Response (acceptat):

```json
{
  "result": { "module_id": "duckdb", "state": "queued" }
}
```

Response (deja descărcat):

```json
{
  "result": { "module_id": "duckdb", "state": "ready" }
}
```

### 8.2 `module_status`

Request:

```json
{
  "method": "module_status",
  "params": { "module_id": "duckdb" },
  "id": 11
}
```

Response:

```json
{
  "result": {
    "module_id": "duckdb",
    "version": "1.1.3",
    "state": "ready",
    "progress": {
      "bytes_downloaded": 16777216,
      "bytes_total": 16777216,
      "bytes_per_second": 0
    },
    "error": null,
    "dependencies_ready": true
  }
}
```

### 8.3 `cancel_download`

Request:

```json
{
  "method": "cancel_download",
  "params": { "module_id": "duckdb" },
  "id": 12
}
```

Response:

```json
{
  "result": { "module_id": "duckdb", "state": "missing" }
}
```

### 8.4 `remove_module`

Request:

```json
{
  "method": "remove_module",
  "params": { "module_id": "duckdb" },
  "id": 13
}
```

Response:

```json
{
  "result": { "module_id": "duckdb", "state": "missing" }
}
```

## 9. Securitate

### 9.1 Catalog semnat

- Catalogul este semnat cu Ed25519.
- Cheia publică este embedded în daemon și nu poate fi suprascrisă prin config.
- Dacă semnătura nu este validă, catalogul este respins.

### 9.2 HTTPS only

- Toate URL-urile din catalog trebuie să înceapă cu `https://`.
- Daemonul rejectează URL-uri `http://` sau locale pentru module.

### 9.3 Checksum per fișier

- Fiecare arhivă are SHA-256.
- Fiecare fișier extras are SHA-256.
- După extracție se verifică toate fișierele required.
- Dacă un fișier nu corespunde, directorul este șters și starea devine `error`.

### 9.4 Sandbox

- Librăriile native descărcate sunt încărcate doar din subdirectoarele `<app-bundle>/downloads/` și `<app-bundle>/models/`.
- Nu se acceptă path-uri absolute de la client.
- Fișierele descărcate primesc permisiuni restrictive (fără `+x` pentru fișiere non-executable).

## 10. Detectarea modulelor lipsă și erori structurate

### 10.1 Detectare în daemon

Când o metodă IPC necesită un modul:

- `query(sql)` necesită `duckdb`.
- `embed(text)` / `search(text)` necesită `text_embedding_model` + `onnx_runtime`.
- `ask(question)` necesită `slm_nl_router` + `onnx_runtime` + `duckdb`.
- Vision features necesită `vision_model` + `onnx_runtime`.

Daemonul verifică starea modulului înainte de execuție. Dacă modulul nu este `ready`, returnează o eroare JSON-RPC structurată.

### 10.2 Formatul erorii

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
      "download_url_hint": "https://cdn.mirage.ai/catalog.json"
    }
  },
  "id": 7
}
```

### 10.3 Mapping în GUI

Clientul KMP primește eroarea și poate afișa:

- Un dialog: "Pentru a rula interogarea SQL este necesar DuckDB Analytics Engine (16 MB). Descarci acum?"
- Un buton care trimite `download_module("duckdb")`.
- Un ecran de progres care ascultă evenimentele `module_status`.

CLI poate afișa un mesaj similar și un flag `--download` care declanșează descărcarea.

## 11. Încărcarea dinamică a librăriilor native

DuckDB și ONNX Runtime sunt librării native care nu sunt linkate static în binarul de bază. Daemonul le încarcă la runtime din directoarele de descărcare.

### 11.1 Strategie de încărcare

1. **Feature gate + runtime detection**
   - Codul Rust definește un trait comun, de ex. `DuckDbEngine` / `OnnxSession`.
   - Implementarea concretă este compilată doar sub feature gate `duckdb` / `onnx`, dar gate-ul este întotdeauna activ pentru build-ul oficial.
   - Când daemonul pornește, verifică dacă librăria nativă există în `downloads/<module-id>/<version>/lib/`.
   - Dacă există, apelează `libloading::Library::new(path)` înainte de prima inițializare a engine-ului.

2. **Platform-specific loading**
   - **macOS**: `libduckdb.dylib` / `libonnxruntime.dylib`; se folosește `dlopen` via crate-ul `libloading`. Necesită ca fișierul să aibă permisiuni `+x` și să nu fie quarantinat (se recomandă semnarea librăriilor la build).
   - **Windows**: `duckdb.dll` / `onnxruntime.dll`; `libloading` apelează `LoadLibraryW`. Dependențele DLL trebuie să fie în același director sau în `PATH`.
   - **Linux**: `libduckdb.so` / `libonnxruntime.so`; `dlopen` caută fișierul exact și librăriile dependente din `LD_LIBRARY_PATH` sau `RPATH`.

3. **Lazy loading**
   - `Analytics` încearcă `load_duckdb()` la prima apelare `query()`.
   - `OnnxEmbedder` încearcă `load_onnx_runtime()` la prima apelare `embed()`.
   - Dacă încărcarea eșuează, daemonul returnează eroare `module_load_failed` cu detalii.

### 11.2 Exemplu de pseudocod

```rust
fn ensure_duckdb_lib(config: &DaemonConfig) -> Result<Library, ModuleError> {
    let path = config.downloads_dir
        .join("duckdb/1.1.3/lib")
        .join(library_filename("duckdb"));
    if !path.exists() {
        return Err(ModuleError::Missing { module_id: "duckdb".into() });
    }
    unsafe { Library::new(&path) }
        .map_err(|e| ModuleError::LoadFailed { path, source: e })
}
```

### 11.3 Rust bindings

- Pentru DuckDB se poate folosi crate-ul `duckdb-rs` cu feature `bundled` dezactivat și variabila de mediu `DUCKDB_LIB_DIR` setată la directorul descărcat.
- Pentru ONNX Runtime se folosește `ort` cu `ORT_LIB_LOCATION` sau `ORT_DYLIB_PATH`.

### 11.4 Fallback determinist (MVP)

Pentru teste fără descărcări, daemonul poate rămâne la embedder deterministic (ADR 006) atunci când `onnx_runtime` lipsește. Acest fallback nu este folosit în producție.

## 12. Testare

- Unit tests pentru parsarea manifestului și a catalogului.
- Mock HTTP server pentru testarea resume și a erorilor de checksum.
- Teste de integrare care descarcă module locale (file://) și verifică starea ready.
- CI rulează testele cu module stub, fără a descărca de pe CDN.

## 13. Referințe

- ADR 012: `.agents/decisions/adr/012-modular-download-manager.md`
- JSON Schema: `.agents/specs/module-manifest-schema.json`
- Technical spec: `.agents/specs/technical-spec.md`
