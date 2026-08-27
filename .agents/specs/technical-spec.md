# Specificație Tehnică de Implementare — Mirage

## 1. Viziune & Context

Mirage este un motor de căutare semantică **local-first** cu arhitectură de daemon. Un proces Rust rulează în fundal și oferă căutare, indexare, analitică SQL și ML local. GUI-ul desktop și CLI-ul sunt clienți care comunică cu daemonul prin IPC. Utilizatorul poate rula Mirage complet local sau se poate conecta la un worker self-hosted/managed pentru volume mari.

## 2. Arhitectură de ansamblu

```
                          ┌───────────────────────────┐
                          │   MIRAGE GUI (KMP App)    │
                          └─────────────┬─────────────┘
                                        │
                                        ▼ (IPC / Socket)
┌───────────────────────────────────────────────────────────────────────────────┐
│                           MIRAGE DAEMON (Rust Background)                       │
│                                                                               │
│  • Vector & Text Index  (LanceDB / Tantivy)                                   │
│  • Tabular & SQL Engine (DuckDB / Parquet — descărcabil la cerere)             │
│  • Embedded ML          (ONNX / Rust SIMD — descărcabil la cerere)            │
│  • Sync Worker          (Local Stream / Remote Cloud Sync)                    │
└───────────────────────────────▲───────────────────────────────────────────────┘
                                │
                                ▼ (IPC / Socket)
                          ┌───────────────────────────┐
                          │  CLI & MCP Clients          │
                          └─────────────┬─────────────┘
                                        │
                                        ▼ (Stdio / Socket)
                          ┌───────────────────────────┐
                          │  TERMINAL COMMANDS          │
                          └───────────────────────────┘
```

## 3. Module de implementare

### Module 1: Mirage Daemon (Rust)

#### 3.1.1 Stack

- **Limbaj**: Rust.
- **Vector search**: LanceDB Rust + Rust SIMD (prezent în binar).
- **OLAP**: DuckDB descărcabil la cerere, activat din Setup Wizard sau la prima interogare tabulară.
- **ML**: ONNX Runtime Rust bindings descărcabili la cerere; modelul de embeddings/vision/SLM vine în `models/`.
- **SLM**: model ONNX mic pentru generare SQL din limbaj natural, pornit direct de daemon fără llama.cpp.
- **IPC**: Unix Domain Sockets (Linux/macOS) + Named Pipes (Windows).
- **Remote sync**: NDJSON streaming over HTTP/2 (MVP), gRPC opțional.
- **Modular download manager**: descarcă binarele/modulele opționale doar când utilizatorul le activează.

#### 3.1.2 Configurare

```yaml
# <app-bundle>/daemon.yaml
data_dir: <app-bundle>/data
models_dir: <app-bundle>/models
downloads_dir: <app-bundle>/downloads
socket_path: <app-bundle>/mirage.sock
log_level: info
modules:
  vector: true        # LanceDB + căutare vectorială, mereu activ
  text: true          # embedder text, descarcă ONNX la primul index/search
  tabular: false      # DuckDB, descărcat la primul query tabular
  sql_generator: false # SLM mic ONNX pentru SQL natural-language
  audio: false
  vision: false
sync:
  workers: []
  interval_sec: 60
```

Toate căile sunt relative la folderul aplicației. La dezinstalare, întreg conținutul (date, modele, descărcări) este șters. Modulele opționale se descarcă doar la cerere, sub controlul utilizatorului.


#### 3.1.3 IPC Protocol

Comenzi trimise ca JSON-RPC 2.0 peste IPC socket:

```json
{
  "jsonrpc": "2.0",
  "method": "search",
  "params": {"query": "contract 2024", "top_k": 10, "hybrid": true},
  "id": 1
}
```

Răspuns:

```json
{
  "jsonrpc": "2.0",
  "result": [
    {"id": "...", "relative_path": "...", "score": 0.95, "source_type": "local"}
  ],
  "id": 1
}
```

#### 3.1.4 Metode IPC

- `search(query, top_k, hybrid)` — vector + BM25 + SQL hibrid.
- `query(sql)` — execută DuckDB SQL; dacă DuckDB nu e descărcat, returnează eroare sau propune descărcarea.
- `ask(question)` — folosește SLM multilingv ONNX pentru a decide intenția (`semantic_search` sau `sql_query`), a rula căutarea sau a genera/executa SQL și a returna un răspuns în limbaj natural.
- `index(path, source_type)` — indexare locală; dacă embedderul ONNX lipsește, propune descărcarea.
- `embed(text)` — produce vector text; fallback deterministic dacă modelul nu e disponibil.
- `download_module(module_id)` — declanșează descărcarea unui modul opțional (`duckdb`, `onnx`, `slm_sql`).
- `status()` — starea daemonului, module active, module disponibile/descărcate.
- `sync(worker_url, code)` — declanșează sincronizare remote.
- `open(path, source_type)` — deschide fișierul prin VFS.

### Module 2: GUI Client (Kotlin Multiplatform + Compose Desktop)

Client vizual care trimite comenzi Daemon-ului prin IPC.

- Floating search window, global hotkey, system tray.
- Preview pentru imagini, video, documente.
- Settings, Add Server, Modular Setup Wizard.
- Nu rulează interogări direct.

### Module 3: CLI Client (Rust/Go)

Binar lightweight pentru terminal:

- `mirage search "query"`
- `mirage query "SELECT ..."`
- `mirage status`
- `mirage mcp serve`

### Module 4: MCP Protocol Support

`mirage mcp serve` expune un server MCP peste stdio:

- Tools: `search`, `query`, `index_path`, `status`.
- Resources: fișiere indexate (read-only).
- Prompts: exemple de interogări.

### Module 5: Remote Worker (Self-Hosted / Managed)

Container Docker pentru procesare la distanță a volumelor mari.

- Expune Admin Web Console pentru managementul cheilor de utilizator.
- Endpoint `/sync/delta` pentru sincronizare delta.
- Suportă S3, GCS, Azure, NAS, Dropbox, Google Drive.

### Module 6: Local ML Models

Modele și runtime-uri ONNX descărcate local în folderul aplicației (ex: `<app-bundle>/downloads/` și `<app-bundle>/models/`):

- **ONNX Runtime**: biblioteca nativă necesară pentru inferență. Se descarcă o singură dată, la primul modul ONNX activat.
- **Text embeddings** (ex: all-MiniLM-L6-v2): descărcat când utilizatorul activează indexarea/căutarea text.
- **Vision embeddings** (CLIP): opțional, descărcat la cerere.
- **SLM pentru generare SQL**: model ONNX mic (fără llama.cpp), pornit direct de daemon pentru a transforma întrebări în SQL.
- **Translator**: opțional.

Descărcarea este explicit aprobată de utilizator în wizard sau în dialog contextual. Toate fișierele se șterg odată cu aplicația.

### Module 7: Modular Setup Wizard

La prima configurare, utilizatorul selectează ce funcționalități vrea activate. Doar nucleul (LanceDB + IPC) vine în binarul de bază; restul se descarcă la cerere:

- [x] Vector & Text Search (LanceDB — inclus)
- [ ] Tabular & SQL Analytics Engine (DuckDB — descărcabil)
- [ ] Audio / Voice Processing Engine (descărcabil)
- [ ] Multi-Modal & Vision Embeddings (ONNX — descărcabil)
- [ ] SLM pentru SQL natural-language (ONNX — descărcabil, fără llama.cpp)

Wizard-ul arată dimensiunea fiecărei descărcări, cere confirmare și gestionează progresul. Modulele pot fi activate ulterior din Settings.

### Module 8: Modular Download Manager (Rust + GUI)

Managerul centralizează descărcările opționale:

- Catalog local cu module disponibile, dimensiuni și URL-uri semnate.
- Descărcare prin HTTPS cu resume, verificare de checksum.
- Stocare în `<app-bundle>/downloads/` și `<app-bundle>/models/`.
- Notificare IPC când un modul a fost activat.
- Ștergerea descărcărilor la dezinstalare.

## 4. Securitate

- IPC local protejat de filesystem permissions.
- Remote workers autentificați cu User API Keys.
- OAuth tokens rămân pe client.
- Model downloads cu confirmare manuală.

## 5. API-uri și contracte

### 5.1 IPC JSON-RPC

Transport: Unix Domain Socket / Named Pipe.
Protocol: JSON-RPC 2.0.

### 5.2 Remote Worker Sync

```http
GET /sync/delta?version={client_last_version}
Authorization: Bearer {user_api_key}
```

Response: NDJSON stream cu recorduri noi.

### 5.3 Record Schema

```json
{
  "id": "string",
  "relative_path": "string",
  "source_type": "enum: local | nas | dropbox | s3 | gdrive",
  "vector": "[float]",
  "updated_at": "timestamp",
  "version": "int"
}
```

## 6. Condiții de acceptanță

- [ ] Daemon Rust pornește și ascultă pe IPC socket.
- [ ] GUI se conectează la daemon și trimite comenzi.
- [ ] CLI `mirage search` returnează rezultate.
- [ ] `mirage query` execută DuckDB SQL.
- [ ] MCP serve funcționează peste stdio.
- [ ] Remote worker expune Admin Web Console și `/sync/delta`.
- [ ] Setup Wizard permite activarea/selectarea modulelor.
- [ ] Modelele ONNX se descarcă doar cu confirmare.
