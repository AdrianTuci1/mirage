# Progress — Mirage

## Stare curentă

- **Data ultimei actualizări:** 2026-08-27
- **Faza curentă:** M1 — Remote indexer pipeline finalizat; pregătire M2 (delta sync protocol)
- **Progres general:** ~50% (T1.1–T1.6, T2.1–T2.2, T3.1–T3.2, T3.4, T4.1, T5.1, T5.3–T5.5 finalizate; T3.3, T4.2–T4.5, T5.2, T6.1–T6.4 rămân pending)

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
| T3.4 | Integrate local LanceDB in KMP (in-memory MVP) | 2026-08-27 |
| T4.1 | Define VfsAdapter interface | 2026-08-26 |
| T5.1 | Build floating search UI in Compose Desktop | 2026-08-27 |
| T5.3 | Implement global hotkey manager (JNativeHook) | 2026-08-27 |
| T5.4 | Implement system tray manager | 2026-08-27 |
| T5.5 | Implement clipboard history manager | 2026-08-27 |

## Task-uri în progres

Niciunul.

## Task-uri următoare (prioritate)

1. **T3.3** — Implement RemoteVaultManager with delta download
4. **T4.2** — Implement LocalFileSystem VFS adapter
5. **T4.3** — Implement Dropbox VFS adapter
6. **T4.4** — Implement Google Drive VFS adapter
7. **T4.5** — Implement NAS/SMB VFS adapter
8. **T5.2** — Implement Add Remote Vault flow with trial gate

## Blockere

- `docker compose up --build` nu a putut fi rulat în mediul curent deoarece daemon-ul Docker nu este pornit. Codul aplicației a fost verificat local (FastAPI pornește, `/health` și `/sync/delta` răspund corect, LanceDB se conectează și schemă este creată conform specificației).
- Buildul KMP (`./gradlew build` din `src/client-kmp/`) necesită un JDK <= 24 (testat cu JDK 21) deoarece Gradle 8.14 nu suportă Java 26.0.1 (unicul JDK instalat pe sistem). Buildul trece când `JAVA_HOME` este setat la un JDK 21 valid.
- Integrarea LanceDB JVM a fost deblocată printr-un store vectorial in-memory MVP (ADR 005). O înlocuire cu LanceDB-JVM sau JNI rămâne posibilă fără schimbări în UI.

## Note pentru următorul agent

- Toate specificațiile sunt în `.agents/specs/`.
- ADR 001 este **Accepted** și justifică alegerea Python 3.11.
- ADR 006 acceptat: pentru MVP, `OnnxEmbedder` folosește vectori pseudo-aleatorii deterministici de dimensiune 384 când nu există un model ONNX în `assets/models/`. Acest lucru permite testarea end-to-end a pipeline-ului fără descărcări de modele.
- Graful de execuție complet este în `.agents/execution-graph/project-graph.json`.
- Design tokens sunt în `.agents/design-tokens/design-system.md`.
- Pipeline-ul Remote Indexer este funcțional în `src/remote-indexer/`. Conectorii local, Dropbox, Google Drive și SMB/NAS sunt implementați; doar `LocalConnector` este complet, restul sunt stub-uri `NotImplementedError`.
- `requirements.txt` include acum `pillow` și `pymupdf`.
- `app/db.py` include `version` în schema LanceDB și helpers `get_latest_version()` / `bump_version()`.
- `app/api/sync.py` implementează `GET /sync/delta` cu autentificare `Authorization: Bearer {passkey}` și răspuns streaming `application/x-ndjson`. Fiecare linie conține o înregistrare completă LanceDB (`id`, `relative_path`, `source_type`, `vector`, `updated_at`, `version`). Headerul `X-Latest-Version` indică versiunea curentă a serverului. Dacă `client_last_version >= current_version`, corpul răspunsului este gol.
- ADR 003 actualizat și acceptat: pentru MVP se folosește NDJSON streaming în loc de fișiere `.lance` brute; gRPC/HTTP2 rămân opțiuni pentru etape ulterioare.
- Teste: `pytest -v` în `src/remote-indexer/` — 9 passed (adăugate `test_sync_delta_returns_records`, `test_sync_delta_empty_when_up_to_date`, `test_sync_delta_rejects_missing_auth`).
- Buildul KMP a fost re-verificat cu `JAVA_HOME=/tmp/jdk-21.0.5+11/Contents/Home ./gradlew build` — BUILD SUCCESSFUL.
- ADR 005 acceptat: pentru clientul KMP se folosește un store vectorial in-memory MVP cu interfața `LocalVectorStore`, lăsând deschisă înlocuirea ulterioară cu LanceDB-JVM sau JNI.
- Modulul `src/client-kmp/src/commonMain/kotlin/search/` conține `VectorRecord`, `SearchResult`, `LocalVectorStore`, `InMemoryVectorStore` și `SearchEngine`.
- `SearchScreen` primește acum `SearchEngine` și afișează rezultate reale într-o listă lazy.
- ADR 004 acceptat: Mirage va fi un launcher global Spotlight/Raycast-style cu global hotkey, fereastră flotantă, system tray și clipboard history.
- Adăugate `GlobalShortcutManager`, `ClipboardManager`, `SystemTrayManager` în `src/client-kmp/src/jvmMain/kotlin/platform/`.
- UI flotant actualizat: shortcut `Ctrl/Cmd + Space`, poziționare pe ecranul activ, bară de stare cu "Start indexing" / "Add vault", fereastră Settings separată.
- Fereastra de onboarding rămâne pentru mai târziu (nu e prioritar acum).
- Pipeline de indexare remote implementat pentru fișiere locale și imagini (T1.4–T1.6 completate); folosește embeddings deterministice MVP.
- Adaptorul VFS local cu thumbnail (T4.2) și UI de preview pentru imagini/video/documente implementate.
- Teste: `pytest -v` — 7 passed; `./gradlew jvmTest` — BUILD SUCCESSFUL.
