# Specificație Tehnică de Implementare — Mirage

## 1. Viziune & Context

Mirage este un motor de căutare semantică **local-first** pentru fișiere personale. Indexarea vectorială poate rula fie complet local (gratuit), fie printr-un container remote sincronizat cu clienți (funcție Pro). Căutarea și deschiderea fișierelor se face direct din sursă, fără proxy prin server.

## 2. System Architecture Overview

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

## 3. Module de implementare

### Module 1: Remote Indexer Setup (Dockerized Service)

#### 3.1.1 Stack Tehnologic

- **Runtime**: Python 3.11 sau Rust binary containerizat.
- **Vector Store**: LanceDB Native Core.
- **ML Inference**: ONNX Runtime (int8 execution provider cu suport AVX2/NEON sau CUDA).
- **Storage Connectors**: fsspec / native SDKs pentru citire read-only din local, NAS (SMB/NFS), S3, Google Drive, Dropbox.

#### 3.1.2 Configurare Docker

```yaml
version: '3.8'

services:
  indexer:
    image: ghcr.io/org/semantic-indexer:latest
    container_name: semantic-indexer
    restart: unless-stopped
    ports:
      - "8080:8080"
    environment:
      - VAULT_NAME=Company-NAS
      - SYNC_INTERVAL_SEC=60
      - MAX_CPU_THREADS=4
      - MAX_RAM_MB=4096
      - SECRET_KEY=generate_on_first_run
    volumes:
      - /mnt/storage/nas_media:/data/source:ro # READ-ONLY!
      - ./vault_db:/data/index
```

#### 3.1.3 Remote Indexer Execution Loop (Pseudo-code)

```python
# 1. Pipeline de ingestie
def run_indexing_pipeline():
    for file in scan_sources_diff():
        # Extragere metadate & frame-uri / audio chunks
        features = extract_features(file)

        # Generare Vectori ONNX
        vector = onnx_session.run(features)

        # Inserare în LanceDB
        lancedb_table.add([{
            "id": file.unique_hash,
            "relative_path": file.relative_path,
            "source_type": file.source_type, # 'nas', 'dropbox', 's3'
            "vector": vector,
            "updated_at": timestamp
        }])

# 2. Endpoint gRPC/HTTP2 de sincronizare a indexului pentru clienți
@app.get("/sync/delta")
def get_delta_index(client_last_version: int):
    # Returnează doar fișierele .lance noi create după versiunea clientului
    return stream_lance_delta_files(from_version=client_last_version)
```

### Module 2: Client Connection & Passkey Handshake

#### 3.2.1 Model de conectare (No Account Required)

Când containerul remote pornește, generează o adresă de conectare numită **Vault URI**:

```
vault://192.168.1.100:8080#vault_id=company_nas&key=sec_pk_9f8a3d12...
```

#### 3.2.2 Implementare client KMP (Kotlin)

```kotlin
data class RemoteVaultConfig(
    val host: String,
    val port: Int,
    val vaultId: String,
    val passkey: String
)

class RemoteVaultManager(private val config: RemoteVaultConfig) {

    suspend fun syncDeltaIndex(localPath: String) {
        val lastVersion = getLocalLanceVersion(localPath)

        // 1. Fetch doar delta-ul de la serverul remote
        val deltaFiles = HttpClient.downloadDelta(
            url = "http://${config.host}:${config.port}/sync/delta?version=$lastVersion",
            headers = mapOf("Authorization" to "Bearer ${config.passkey}")
        )

        // 2. Aplicare delta în baza de date locală LanceDB
        LanceDBNative.applyDelta(localPath, deltaFiles)
    }
}
```

### Module 3: Virtual File System (VFS) & Direct Fetching

Clientul caută în vectorii sincronizați, dar atunci când deschide fișierul sau generează un preview, nu folosește serverul remote ca proxy, ci se conectează direct la sursă.

#### 3.3.1 Interfața VFS (Kotlin)

```kotlin
interface VfsAdapter {
    suspend fun fetchThumbnail(relativePath: String): ByteArray
    suspend fun openFile(relativePath: String)
}

class DropboxVfsAdapter(private val oauthToken: String) : VfsAdapter {
    override suspend fun fetchThumbnail(relativePath: String): ByteArray {
        return dropboxClient.files().getThumbnail(relativePath)
    }

    override suspend fun openFile(relativePath: String) {
        val localTempFile = dropboxClient.files().download(relativePath)
        Desktop.getDesktop().open(localTempFile)
    }
}

class NasSmbVfsAdapter(private val smbCredentials: SmbCredentials) : VfsAdapter {
    override suspend fun openFile(relativePath: String) {
        val fullPath = Path.of(smbCredentials.rootPath, relativePath)
        Desktop.getDesktop().open(fullPath.toFile())
    }
}
```

### Module 4: Global Search UI & OS Integration (Spotlight/Raycast-style)

Clientul nu este o aplicație desktop tradițională cu fereastră permanentă, ci un **launcher global** similar cu Spotlight pe macOS sau Raycast. Utilizatorul apelează o scurtătură globală, tastează un query, primește rezultate și acționează asupra lor (deschide, copiază, previzualizează).

#### 3.4.1 Componente OS

- **Global Shortcut Manager**: ascultă o combinație globală de taste (`Ctrl + Space` pe Windows/Linux, `Cmd + Space` pe macOS) folosind JNativeHook.
- **Floating Search Window**: fereastră compactă, centrată pe ecranul activ (cursorul mouse-ului), fără decorațiuni, afișată/ascunsă la activarea scurtăturii.
- **System Tray**: iconiță în bara de meniu/sistem cu opțiuni Show / Settings / Quit.
- **Clipboard Manager**: istoric al clipboard-ului local, indexabil și căutabil (opțional activabil).
- **Settings Window**: fereastră separată pentru vaulturi, indexare, scurtături și licențiere.
- **Empty state**: bară de stare sub search input care arată procentul indexat și butoane rapide "Start indexing" / "Add vault".

#### 3.4.2 Stack UI

- **Compose Multiplatform (Desktop)**: UI declarativ, un singur codebase pentru macOS, Windows, Linux.
- **JNativeHook**: librărie nativă cross-platform pentru global hotkeys.
- **AWT SystemTray + Clipboard**: API-uri built-in Java pentru tray icon și clipboard.

#### 3.4.3 Exemplu de flow

```kotlin
// 1. Utilizatorul apasă Cmd/Ctrl + Shift + Space
GlobalShortcutManager.register("Cmd+Shift+Space") {
    FloatingSearchWindow.show()
}

// 2. Tastează query; clientul caută în LanceDB local
val results = searchEngine.query(query)

// 3. Selectează un rezultat și apasă Enter -> deschide fișierul
VfsManager.open(result.relativePath, result.sourceType)

// 4. Esc sau click în afara ferestrei -> ascunde fereastra
FloatingSearchWindow.hide()
```

#### 3.4.4 Atenție la dependențe Compose

Imports-urile sunt `androidx.compose.*`, dar artifactele rezolvate sunt `org.jetbrains.compose.*` (multiplatform desktop). Nu folosim Jetpack Compose Android-only.

## 4. Cerințe nefuncționale

- **Local-first**: indexarea locală este gratuită și fără limite de dimensiune.
- **Zero-Trust / No-Account**: nu se cere autentificare pe servere centrale.
- **Performanță**: inferență ONNX int8, suport AVX2/NEON/CUDA.
- **Securitate**: conectorii read-only; token-urile private rămân pe client.
- **Portabilitate**: Kotlin Multiplatform pentru desktop (Windows, macOS, Linux).
- **Discreție OS**: aplicația rulează în background, tray icon, global hotkey, fereastră flotantă.

## 5. API-uri și contracte

### 5.1 Vault URI

```
vault://{host}:{port}#vault_id={id}&key={passkey}
```

### 5.2 Delta Sync Endpoint

```http
GET /sync/delta?version={client_last_version}
Authorization: Bearer {passkey}
```

Response: stream de fișiere `.lance` noi.

### 5.3 LanceDB Record Schema

```json
{
  "id": "string (unique hash)",
  "relative_path": "string",
  "source_type": "enum: nas | dropbox | s3 | gdrive | local",
  "vector": "[float]",
  "updated_at": "timestamp"
}
```

## 6. Condiții de acceptanță

- [ ] Remote indexer rulează în Docker cu volum read-only.
- [ ] Endpoint-ul `/sync/delta` returnează corect delta-ul de fișiere `.lance`.
- [ ] Clientul KMP poate parse Vault URI și poate sincroniza local LanceDB.
- [ ] VFS poate deschide direct fișiere din local, Dropbox, Google Drive și NAS/SMB.
- [ ] Licența offline este validată prin cheie ED25519.
- [ ] Global hotkey deschide fereastra flotantă de căutare pe macOS/Windows/Linux.
- [ ] System tray icon permite Show/Settings/Quit.
- [ ] Clipboard history este indexabil și căutabil (opțional).
