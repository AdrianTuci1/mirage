# System Architecture Overview — Mirage

## 1. Diagramă de ansamblu

```
                       +-----------------------------------+
                       |    REMOTE INDEXER (Docker Container)|
                       |  (Runs ONNX/LanceDB on CPU/GPU)   |
                       +-----------------+-----------------+
                                         |
                                         | Delta Vector Sync (gRPC/HTTP2)
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

## 2. Componente principale

### 2.1 Remote Indexer

Container Docker care rulează pe un server cu acces la sursele de date. Responsabilități:

- Scanează sursele configurate (local, NAS, cloud) în mod read-only.
- Extrage feature-uri (text, imagine, audio) și generează embedding-uri prin ONNX Runtime.
- Stochează vectorii și metadatele în LanceDB.
- Expune endpoint-ul `/sync/delta` pentru sincronizarea clienților.

### 2.2 Kotlin Multiplatform Client

Aplicație desktop (Compose Desktop) care rulează pe Windows, macOS și Linux. Responsabilități:

- Parsează Vault URI și se conectează la Remote Indexer.
- Descarcă delta-ul de index și îl aplică în LanceDB local.
- Caută în indexul local folosind vectori de interogare.
- Deschide fișierele direct din sursă prin adaptoare VFS.
- Validează licența Pro offline.

### 2.3 Virtual File System (VFS)

Strat de abstractizare care permite clientului să acceseze fișierele direct de la sursă:

- `LocalVfsAdapter`: acces direct pe disc.
- `DropboxVfsAdapter`: API Dropbox cu token OAuth.
- `GoogleDriveVfsAdapter`: Drive REST API v3.
- `NasSmbVfsAdapter`: SMB prin `smbj` / JCIFS sau cale de rețea directă.

## 3. Fluxuri de date

### 3.1 Indexare remote

```
Sursă (read-only) -> Extract features -> ONNX inference -> LanceDB -> Delta files
```

### 3.2 Sincronizare client

```
Client -> GET /sync/delta?version=X -> Server stream .lance delta -> Local LanceDB
```

### 3.3 Căutare și deschidere

```
User query -> Embedding local/remote -> LanceDB ANN search -> Rezultate -> VFS open -> Aplicație nativă
```

## 4. Decizii cheie

- **LanceDB**: vector store nativ, suportă fișiere delta și este ușor de sincronizat.
- **ONNX Runtime**: rulează local fără dependențe de cloud.
- **Kotlin Multiplatform**: un singur codebase pentru desktop.
- **Direct fetching**: serverul nu servește conținut, reducând latența și costurile.
- **Offline licensing**: respectă filozofia no-account.

## 5. Securitate

- Volumele sursă sunt montate read-only în container.
- Token-urile OAuth rămân pe client.
- Licențele sunt validate offline cu ED25519.
- Comunicația remote poate fi securizată prin TLS (recomandat pentru LAN/WAN).
