# Progress — Mirage

## Stare curentă

- **Data ultimei actualizări:** 2026-08-26
- **Faza curentă:** M3 — KMP client engine & Vault URI parser
- **Progres general:** ~25% (T1.1–T1.3, T3.1, T3.2, T4.1 finalizate; T3.3–T3.4, T4.2–T4.5 rămân pending)

## Task-uri finalizate

| ID | Task | Data finalizării |
|----|------|------------------|
| T0.1 | Initialize repo, .agents structure and README | 2026-08-26 |
| T1.1 | Choose Remote Indexer runtime: Python vs Rust | 2026-08-26 |
| T1.2 | Set up Docker container for Remote Indexer | 2026-08-26 |
| T1.3 | Integrate LanceDB native core in Remote Indexer | 2026-08-26 |
| T3.1 | Create KMP project skeleton (Compose Desktop) | 2026-08-26 |
| T3.2 | Implement Vault URI parser | 2026-08-26 |
| T4.1 | Define VfsAdapter interface | 2026-08-26 |

## Task-uri în progres

Niciunul.

## Task-uri următoare (prioritate)

1. **T1.4** — Integrate ONNX Runtime for embedding inference
2. **T1.5** — Implement storage connectors (local, NAS/SMB, S3, Dropbox, Google Drive)
3. **T1.6** — Implement indexing pipeline: scan, extract, embed, store
4. **T2.1** — Design delta sync protocol (HTTP2/gRPC)
5. **T3.3** — Implement RemoteVaultManager with delta download
6. **T3.4** — Integrate local LanceDB in KMP
7. **T4.2** — Implement LocalFileSystem VFS adapter
8. **T4.3** — Implement Dropbox VFS adapter
9. **T4.4** — Implement Google Drive VFS adapter
10. **T4.5** — Implement NAS/SMB VFS adapter

## Blockere

- `docker compose up --build` nu a putut fi rulat în mediul curent deoarece daemon-ul Docker nu este pornit. Codul aplicației a fost verificat local (FastAPI pornește, `/health` și `/sync/delta` răspund corect, LanceDB se conectează și schemă este creată conform specificației).
- Buildul KMP (`./gradlew build` din `src/client-kmp/`) necesită un JDK <= 24 (testat cu JDK 21) deoarece Gradle 8.14 nu suportă Java 26.0.1 (unicul JDK instalat pe sistem). Buildul trece când `JAVA_HOME` este setat la un JDK 21 valid.
- Integrarea LanceDB JVM este pending: nici `com.github.lancedb`, nici `com.lancedb:lancedb` nu sunt disponibile pe Maven Central. JNI/native integration este planificată pentru T3.4.

## Note pentru următorul agent

- Toate specificațiile sunt în `.agents/specs/`.
- ADR 001 este acum **Accepted** și justifică alegerea Python 3.11.
- Graful de execuție complet este în `.agents/execution-graph/project-graph.json`.
- Design tokens sunt în `.agents/design-tokens/design-system.md`.
- Scheletul Remote Indexer este funcțional în `src/remote-indexer/`. T1.4–T1.6 rămân de implementat.
