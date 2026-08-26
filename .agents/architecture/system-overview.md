# System Architecture Overview — Mirage

## 1. Diagramă de ansamblu

```
                          ┌───────────────────────────┐
                          │   MIRAGE GUI (KMP App)    │
                          └─────────────┬─────────────┘
                                        │
                                        ▼ (IPC / Socket)
┌───────────────────────────────────────────────────────────────────────────────┐
│                           MIRAGE DAEMON (Background)                          │
│                                                                               │
│  • Vector & Text Index  (LanceDB / Tantivy)                                   │
│  • Tabular & SQL Engine (DuckDB / Parquet)                                    │
│  • Embedded ML          (ONNX Runtime / Rust SIMD)                            │
│  • Sync Worker          (Local Stream / Remote Cloud Sync)                    │
└───────────────────────────────▲───────────────────────────────────────────────┘
                                │
                                ▼ (IPC / Socket)
                          ┌───────────────────────────┐
                          │  CLI & MCP Clients          │
                          │  (Octomus, Claude, etc.)    │
                          └─────────────┬─────────────┘
                                        │
                                        ▼ (Stdio / Socket)
                          ┌───────────────────────────┐
                          │  TERMINAL COMMANDS        │
                          │  mirage query / search    │
                          └───────────────────────────┘
```

## 2. Componente principale

### 2.1 Mirage Daemon (Rust)

Proces de fundal care rulează continuu, chiar și când GUI-ul este închis. Responsabilități:

- Indexare locală și embedded (text, vectori, tabular).
- Motor de căutare hibrid (vectorial + BM25 + SQL).
- Sync worker pentru date locale și remote.
- Expunere IPC prin Unix Domain Sockets (Linux/macOS) și Named Pipes (Windows).
- Încărcare modele ONNX locale (embeddings, vision, translator, SLM).
- Execuție DuckDB pentru analitică tabulară.

### 2.2 Mirage GUI (Kotlin Multiplatform + Compose Desktop)

Client vizual care se conectează la Daemon prin IPC. Responsabilități:

- Global hotkey, floating search window, system tray.
- Afișare rezultate și preview (imagine, video, document).
- Settings, Add Server, Modular Setup Wizard.
- Nu execută interogări direct; trimite comenzi Daemon-ului.

### 2.3 Mirage CLI (Rust sau Go)

Binar lightweight care comunică cu Daemon-ul prin IPC. Comenzi:

- `mirage search "query"` — căutare hibridă.
- `mirage query "SELECT ..."` — interogare DuckDB.
- `mirage status` — stare daemon, memorie, workeri.
- `mirage mcp serve` — servește protocolul MCP via stdio.

### 2.4 MCP Clients

Agenți AI (Octomus, Claude, etc.) care folosesc protocolul Model Context Protocol pentru a interoga datele locale prin `mirage mcp serve`.

### 2.5 Worker (Self-Hosted / Managed)

Proces remote care indexează volume mari aproape de sursă (S3, NAS, cloud) și trimite doar delta-index comprimat către Daemon.

## 3. IPC Layer

| Platform | Mecanism | Path exemplu |
|----------|----------|--------------|
| Linux / macOS | Unix Domain Socket | `~/.mirage/mirage.sock` sau `/var/run/mirage.sock` |
| Windows | Named Pipe | `\\.\pipe\mirage_engine` |

Securitate:
- Sockets sunt protejate de POSIX permissions / Windows ACL.
- Doar procesele aceluiași utilizator se pot conecta.
- Evită expunerea unui port TCP localhost vulnerabil la cross-site port scanning.

## 4. Fluxuri de date

### 4.1 Căutare GUI

```
User query → GUI → IPC → Daemon → Vector Search + BM25 + DuckDB → IPC → GUI → Rezultate
```

### 4.2 Căutare CLI

```
Terminal → mirage search → IPC → Daemon → Search → Terminal output
```

### 4.3 Analitică SQL

```
Natural language → SLM → SQL → DuckDB → Date brute (Parquet/CSV) → Rezultat exact
```

### 4.4 Indexare smart routing

```
Sursă mică/medie → Daemon procesează local
Sursă mare     → Daemon → Remote Worker → Delta index → Daemon
```

### 4.5 Sync

```
Worker / Local Indexer → LanceDB / Parquet → Delta → Daemon → Local store
```

## 5. Stack tehnologic actualizat

- **Daemon**: Rust (performanță, memory safety, SIMD).
- **GUI**: Kotlin Multiplatform + Compose Desktop.
- **CLI**: Rust (poate împărtăși librării cu daemonul).
- **Vector store**: LanceDB pentru fișiere delta; DuckDB/Tantivy pentru text search.
- **OLAP**: DuckDB pentru Parquet/CSV/SQLite/JSON.
- **ML**: ONNX Runtime pentru embeddings, vision, translator, SLM.
- **IPC**: Unix Domain Sockets / Named Pipes.
- **Remote sync**: HTTP/2 NDJSON (MVP), gRPC opțional.

## 6. Securitate

- IPC local securizat prin filesystem permissions.
- Remote workers se autentifică cu chei de utilizator gestionate din Admin Web Console (self-hosted) sau Dashboard SaaS (managed).
- Token-uri OAuth pentru cloud (Dropbox, Drive) rămân pe client.
- Model downloads necesită confirmare manuală.

## 7. Decizii cheie

- **Daemon + IPC**: GUI și CLI sunt clienți, nu rulează interogări.
- **DuckDB**: motor OLAP embedded pentru analitică tabulară rapidă.
- **Rust pentru daemon**: performanță maximă și acces nativ la SIMD.
- **MCP**: integrare standard cu agenți AI.
- **Wizard modular**: utilizatorul controlează ce module și modele activează.
