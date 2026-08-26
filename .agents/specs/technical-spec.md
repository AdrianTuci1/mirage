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
│  • Tabular & SQL Engine (DuckDB / Parquet)                                    │
│  • Embedded ML          (ONNX / Rust SIMD)                                    │
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
- **Vector search**: LanceDB Rust + Rust SIMD.
- **OLAP**: DuckDB embedded.
- **ML**: ONNX Runtime Rust bindings.
- **IPC**: Unix Domain Sockets (Linux/macOS) + Named Pipes (Windows).
- **Remote sync**: NDJSON streaming over HTTP/2 (MVP), gRPC opțional.

#### 3.1.2 Configurare

```yaml
# ~/.mirage/daemon.yaml
data_dir: ~/.mirage/data
models_dir: ~/.mirage/models
socket_path: ~/.mirage/mirage.sock
log_level: info
modules:
  vector: true
  text: true
  tabular: true
  audio: false
  vision: false
sync:
  workers: []
  interval_sec: 60
```

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
- `query(sql)` — execută DuckDB SQL.
- `index(path, source_type)` — indexare locală.
- `status()` — starea daemonului.
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

Modele ONNX descărcate local în `~/.mirage/models/`:

- Text embeddings (all-MiniLM-L6-v2).
- Vision embeddings (CLIP, opțional).
- SLM pentru generare SQL (opțional).
- Translator (opțional).

Descărcarea este explicit aprobată de utilizator în wizard.

### Module 7: Modular Setup Wizard

La prima configurare, utilizatorul selectează:

- [x] Vector & Text Indexing (LanceDB / Tantivy)
- [x] Tabular & SQL Analytics Engine (DuckDB)
- [ ] Audio / Voice Processing Engine
- [ ] Multi-Modal & Vision Embeddings
- [ ] SLM pentru SQL natural-language

Fiecare modul descarcă doar binarele necesare.

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
