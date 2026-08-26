# Mirage

Mirage este un motor de căutare semantică **local-first** pentru fișiere personale. Indexarea vectorială rulează local sau printr-un container remote sincronizat; fișierele se deschid direct din sursă, fără proxy prin server.

## Arhitectură

```
                       +-----------------------------------+
                       |    REMOTE INDEXER (Docker Container)|
                       |  (Runs ONNX/LanceDB on CPU/GPU)   |
                       +-----------------+-----------------+
                                         |
                                         | Delta Vector Sync (HTTP2)
                                         v
+-----------------------------------------------------------------------------------+
|                        KOTLIN MULTIPLATFORM CLIENT ENGINE                         |
|                                                                                   |
|  [Search Engine (LanceDB Local)] <--- Reads synced vector files (.lance)          |
|  [VFS Manager (Direct Fetch)]    ---> Connects via user's private tokens           |
+-------------------+--------------------+--------------------+---------------------+
                    |                    |                    |
                    v                    v                    v
            +---------------+    +---------------+    +---------------+
            |  Local File   |    |    Dropbox    |    |   Google Drive|
            |   System      |    |  (OAuth API)  |    |  (REST API v3)|
            +---------------+    +---------------+    +---------------+
```

## Module

- `src/remote-indexer/` — Container Docker pentru indexare remote.
- `src/client-kmp/` — Aplicație desktop Kotlin Multiplatform + Compose.
- `src/shared/` — Cod și scripturi comune (licențiere, utils).

## Monetizare

| Tier | Preț | Ce include? |
|------|------|-------------|
| Community / Local | Gratuit | Indexare 100% locală, fără limite. |
| Pro / Remote Vaults | $10 / device | Conectare la NAS, Dropbox, Drive, S3. |
| Enterprise Team | $49 / server | Remote Indexer cu RBAC, clienți nelimitați. |

Licențierea este **offline**, bazată pe ED25519 — fără conturi sau server central.

## Dezvoltare locală

### Remote Indexer

```bash
cd src/remote-indexer
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
uvicorn app.main:app --host 127.0.0.1 --port 8080
```

Endpoint-uri:
- `GET /health`
- `GET /sync/delta?version={n}`

### Client desktop (KMP + Compose Desktop)

Necesită JDK 21 (Gradle 8.14 nu suportă JDK 26). Dacă sistemul are doar JDK 26:

```bash
# Opțiunea A: folosește un JDK 21 descărcat temporar
export JAVA_HOME=/tmp/jdk-21.0.5+11/Contents/Home

# Opțiunea B: instalează Temurin 21
cd src/client-kmp
./gradlew build
```

## Documentație

Toate specificațiile, deciziile și planul de execuție sunt în folderul `.agents/`.

- `.agents/specs/technical-spec.md` — Specificația tehnică.
- `.agents/specs/pricing-monetization.md` — Strategia de monetizare.
- `.agents/execution-graph/project-graph.json` — Graful de execuție.
- `.agents/architecture/system-overview.md` — Arhitectura de ansamblu.
- `.agents/decisions/` — ADR-uri și comparații.

## Stare

Proiect în fază de planificare. Vezi `.agents/progress/progress.md`.
