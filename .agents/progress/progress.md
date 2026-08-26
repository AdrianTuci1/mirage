# Progress — Mirage

## Stare curentă

- **Data ultimei actualizări:** 2026-08-27
- **Faza curentă:** M9 — Rust Core Daemon + IPC
- **Progres general:** ~70% (T1.1–T1.6, T2.1–T2.2, T3.1–T3.4, T4.1–T4.2, T5.1–T5.5, T8.1–T8.4, T9.1–T9.4 finalizate; T4.3–T4.5, T9.5–T9.6, T10.1–T10.3, T11.1–T11.2, T12.1–T12.2 rămân pending)

## Task-uri finalizate

| ID | Task | Data finalizării |
|----|------|------------------|
| T0.1 | Initialize repo, .agents structure and README | 2026-08-26 |
| T1.1 | Choose Remote Indexer runtime: Python vs Rust | 2026-08-26 |
| T1.2 | Set up Docker container for Remote Indexer | 2026-08-26 |
| T1.3 | Integrate LanceDB native core in Remote Indexer | 2026-08-26 |
| T1.4 | Integrate ONNX Runtime for embedding inference | 2026-08-27 |
| T1.5 | Implement storage connectors (local + stubs) | 2026-08-27 |
| T1.6 | Implement indexing pipeline: scan, extract, embed, store | 2026-08-27 |
| T2.1 | Design delta sync protocol (HTTP2/gRPC) | 2026-08-27 |
| T2.2 | Implement /sync/delta endpoint | 2026-08-27 |
| T3.1 | Create KMP project skeleton (Compose Desktop) | 2026-08-26 |
| T3.2 | Implement Vault URI parser | 2026-08-26 |
| T3.3 | Implement RemoteVaultManager with delta download | 2026-08-27 |
| T3.4 | Integrate local LanceDB in KMP (in-memory MVP) | 2026-08-27 |
| T4.1 | Define VfsAdapter interface | 2026-08-26 |
| T5.1 | Build floating search UI in Compose Desktop | 2026-08-27 |
| T5.2 | Implement Add Server flow with server code | 2026-08-27 |
| T5.3 | Implement global hotkey manager (JNativeHook) | 2026-08-27 |
| T5.4 | Implement system tray manager | 2026-08-27 |
| T5.5 | Implement clipboard history manager | 2026-08-27 |
| T8.1 | Add ONNX Runtime Java dependency to KMP | 2026-08-27 |
| T8.2 | Implement local text embedder for KMP | 2026-08-27 |
| T8.3 | Wire local embedder into SearchEngine for real vector search | 2026-08-27 |
| T8.4 | Add local vision and translator stubs | 2026-08-27 |
| T9.1 | Set up Rust daemon project skeleton | 2026-08-27 |
| T9.2 | Implement IPC server (Unix socket + named pipe) | 2026-08-27 |
| T9.3 | Integrate LanceDB Rust + vector search | 2026-08-27 |
| T9.4 | Implement search RPC method | 2026-08-27 |

## Task-uri în progres

Niciunul.

## Task-uri următoare (prioritate)

1. **T9.5** — Integrate ONNX Runtime Rust for local embeddings
2. **T9.6** — Implement DuckDB analytics engine
3. **T10.1** — Build Mirage CLI binary
6. **T10.2** — Implement mirage search / query / status commands
7. **T10.3** — Implement mirage mcp serve
8. **T4.3** — Implement Dropbox VFS adapter
9. **T4.4** — Implement Google Drive VFS adapter
10. **T4.5** — Implement NAS/SMB VFS adapter

## Blockere

- `docker compose up --build` nu a putut fi rulat în mediul curent deoarece daemon-ul Docker nu este pornit. Codul aplicației a fost verificat local (FastAPI pornește, `/health` și `/sync/delta` răspund corect, LanceDB se conectează și schemă este creată conform specificației).
- Buildul KMP (`./gradlew build` din `src/client-kmp/`) necesită un JDK <= 24 (testat cu JDK 21) deoarece Gradle 8.14 nu suportă Java 26.0.1 (unicul JDK instalat pe sistem). Buildul trece când `JAVA_HOME` este setat la un JDK 21 valid.
- Integrarea LanceDB JVM a fost deblocată printr-un store vectorial in-memory MVP (ADR 005). O înlocuire cu LanceDB-JVM sau JNI rămâne posibilă fără schimbări în UI.

## Note pentru următorul agent

- Toate specificațiile sunt în `.agents/specs/`.
- ADR 001 este **Accepted** și justifică alegerea Python 3.11.
- ADR 006 acceptat: pentru MVP, `OnnxEmbedder` folosește vectori pseudo-aleatorii deterministici de dimensiune 384 când nu există un model ONNX în `~/.mirage/models/`. Acest lucru permite testarea end-to-end a pipeline-ului fără descărcări de modele.
- Implementat T8.1–T8.4: ONNX Runtime 1.19.0 adăugat în `src/client-kmp/build.gradle.kts`, `LocalEmbedder`/ `OnnxRuntimeEmbedder` în `src/client-kmp/src/jvmMain/kotlin/ai/`, interfața `LocalEmbedder` expusă în `commonMain` pentru `SearchEngine`, stub-uri `LocalVision` și `LocalTranslator` adăugate. Build și `jvmTest` trec.
- Graful de execuție complet este în `.agents/execution-graph/project-graph.json`.
- Design tokens sunt în `.agents/design-tokens/design-system.md`.
- Pipeline-ul Remote Indexer este funcțional în `src/remote-indexer/`. Conectorii local, Dropbox, Google Drive și SMB/NAS sunt implementați; doar `LocalConnector` este complet, restul sunt stub-uri `NotImplementedError`.
- `requirements.txt` include acum `pillow` și `pymupdf`.
- `app/db.py` include `version` în schema LanceDB și helpers `get_latest_version()` / `bump_version()`.
- `app/api/sync.py` implementează `GET /sync/delta` cu autentificare `Authorization: Bearer {passkey}` și răspuns streaming `application/x-ndjson`. Fiecare linie conține o înregistrare completă LanceDB (`id`, `relative_path`, `source_type`, `vector`, `updated_at`, `version`). Headerul `X-Latest-Version` indică versiunea curentă a serverului. Dacă `client_last_version >= current_version`, corpul răspunsului este gol.
- ADR 003 actualizat și acceptat: pentru MVP se folosește NDJSON streaming în loc de fișiere `.lance` brute; gRPC/HTTP2 rămân opțiuni pentru etape ulterioare.
- Teste: `pytest -v` în `src/remote-indexer/` — 9 passed (adăugate `test_sync_delta_returns_records`, `test_sync_delta_empty_when_up_to_date`, `test_sync_delta_rejects_missing_auth`).
- Buildul KMP a fost re-verificat cu `JAVA_HOME=/tmp/jdk-21.0.5+11/Contents/Home ./gradlew build` — BUILD SUCCESSFUL.
- Implementat T9.1–T9.2: proiect Rust `mirage-daemon` în `src/daemon/` cu `Cargo.toml`, `main.rs`, `lib.rs`, `config.rs`, `logging.rs`, `models/`, și modulul `ipc/` (`server.rs`, `protocol.rs`, `client.rs`, `mod.rs`).
- Configul `DaemonConfig` folosește căi relative la directorul executabilului (`std::env::current_exe().parent()`), nu `~/.mirage`, cu `load()` / `save()` YAML și JSON.
- IPC server implementează JSON-RPC 2.0 peste Unix Domain Socket (macOS/Linux) și Named Pipes (Windows), cu handler-e `ping` → `pong` și `status` → `{status: ok, version: 0.1.0}`.
- Test de integrare `tests/ipc_ping.rs` pornește daemon-ul, conectează socket-ul și verifică răspunsul `pong`; trece pe macOS.
- Build și teste: `cargo build` și `cargo test` trec în `src/daemon/` (3 unit tests + 1 integration test).
- ADR 005 acceptat: pentru clientul KMP se folosește un store vectorial in-memory MVP cu interfața `LocalVectorStore`, lăsând deschisă înlocuirea ulterioară cu LanceDB-JVM sau JNI.
- Modulul `src/client-kmp/src/commonMain/kotlin/search/` conține `VectorRecord`, `SearchResult`, `LocalVectorStore`, `InMemoryVectorStore` și `SearchEngine`.
- `SearchScreen` primește acum `SearchEngine` și afișează rezultate reale într-o listă lazy.
- ADR 004 acceptat: Mirage va fi un launcher global Spotlight/Raycast-style cu global hotkey, fereastră flotantă, system tray și clipboard history.
- Adăugate `GlobalShortcutManager`, `ClipboardManager`, `SystemTrayManager` în `src/client-kmp/src/jvmMain/kotlin/platform/`.
- UI flotant actualizat: shortcut `Ctrl/Cmd + Space`, poziționare pe ecranul activ, bară de stare cu "Start indexing" / "Add vault", fereastră Settings separată.
- Fereastra de onboarding rămâne pentru mai târziu (nu e prioritar acum).
- Pipeline de indexare remote implementat pentru fișiere locale și imagini (T1.4–T1.6 completate); folosește embeddings deterministice MVP.
- Adaptorul VFS local cu thumbnail (T4.2) și UI de preview pentru imagini/video/documente implementate.
- Implementat T3.3: `RemoteVaultManager` sincronizează delta NDJSON de la `/sync/delta` și aplică înregistrările în `LocalVectorStore` prin `SearchEngine`. Teste adăugate pentru sync cu înregistrări, delta gol și eroare de autentificare.
- `LocalVectorStore` expune acum `upsertAll()` și `latestVersion()`; `VectorRecord` include câmpul `version`; `SearchEngine` expune store-ul subiacent.
- UI actualizat cu buton "Sync" în bara de stare; la finalizare se reîmprospătează rezultatele.
- Implementat T5.2: flow-ul Add Server cu URL + code sau Vault URI complet; `ServerConnection` abstractizează conexiunea, iar `RemoteVaultManager` folosește flag-ul HTTPS. SettingsWindow include acum secțiunea Servers cu serverele conectate.
- Planul general a fost extins: arhitectură Core Daemon Rust + IPC pentru GUI/CLI/MCP, DuckDB analytics, modular setup wizard, Admin Web Console pentru worker self-hosted. Vezi ADR-urile 008, 009, 010, 011.
- Constrângeri noi: DuckDB pre-impachetat în binar; datele locale și modelele ONNX se stochează în folderul aplicației (nu în `~/.mirage` sau `Documents`); la dezinstalare totul este șters; nu se depinde de llama.cpp / GGUF.
- Teste: `pytest -v` — 7 passed; `./gradlew jvmTest` — BUILD SUCCESSFUL.
- Implementat T9.3–T9.4: `LanceDbStore` în `src/daemon/src/db.rs`, `search` și `index` RPC în `src/daemon/src/ipc/server.rs`, integrat în `main.rs`. Căutarea folosește LanceDB pentru stocare și similaritate cosinus în Rust (brute-force MVP fără index vectorial). Test de integrare `tests/search_rpc.rs` verifică răspuns gol inițial, indexare și căutare cu scor.
- Build și teste: `cargo build` și `cargo test` trec în `src/daemon/` (5 unit tests + 2 integration tests). A fost necesară ridicarea timeout-ului în `tests/ipc_ping.rs` la 30s din cauza timpului de inițializare LanceDB la primul pornire.
- Notă: `lancedb = "0.37"` depinde intern de `arrow` 58, așa că am folosit `arrow-array/arrow-schema/arrow-cast/arrow-buffer = "58"` în loc de "53" pentru a evita conflictele de versiune. Compilatorul `protoc` a fost descărcat manual în `/tmp/protoc` deoarece LanceDB are nevoie de el la build.
