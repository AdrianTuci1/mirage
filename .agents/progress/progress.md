# Progress — Mirage

## Stare curentă

- **Data ultimei actualizări:** 2026-08-31
- **Faza curentă:** Aplicația a fost adusă la UI-ul din Penpot; indexarea e inițiată de utilizator și folosește embeddings CLIP reali (text + imagine în același spațiu 512-d).
- **Progres general:** ~99% pentru funcționalitatea locală standalone
- **Reguli de produs (ADR 013):** indexarea nu pornește automat nicăieri; procesarea locală sau pe worker; credential-ele nu părăsesc device-ul.
- **Wizard/onboarding:** exclus momentan; modulele se gestionează direct din Settings → Modules.
- **Build artifacts:** directorul `src/daemon_next/target/` a fost curățat (~20GB eliberați).
- **Buget memorie indexare:** daemonul respectă `memory_budget_mb` (default 3072 MB); batching embeddings, upsert LanceDB și procesare cloud sunt limitate de acest buget.
- **Build artifacts:** directorul `src/daemon_next/target/` a fost curățat din nou (~20.5GB eliberați); nu au rămas fișiere `.log`.
- **Profil dev Cargo:** `src/daemon_next/Cargo.toml` are acum `[profile.dev] debug = "line-tables-only"` + `[profile.dev.package."*"] debug = false`;
  înainte, DWARF complet pentru 733 de crate umfla `target/`. Greutatea binarului rămas nu e debug info, ci cod static: `libduckdb.a` 236 MB (feature `bundled`),
  `lance` 227 MB, `lance-index` 105 MB, `aws-sdk-s3` 93 MB, `ort-sys` 78 MB (ONNX Runtime static).
  Greutățile CLIP (148 MB) și tabela LanceDB sunt fișiere runtime în `models_dir` / `data_dir/lancedb`, nu sunt linkuite în binar.

## Ce funcționează local (finalizat)

| Componentă | Status | Note |
|------------|--------|------|
| Rust daemon (`src/daemon_next/`) | ✅ | IPC, search, indexare, apps, module manager, conectori cloud, file watcher |
| CLI `mirage` | ✅ | search, query, status, ask, module, auto-spawn daemon |
| KMP GUI (cod) | ✅ | floating search, tray, hotkey, IPC, module status, cloud badge, open URL, Shift+Enter download |
| Conectori cloud | ✅ | S3/R2, Dropbox, Google Drive, SMB (metadata + open URL, download explicit) |
| Clipboard history | ✅ | Istoric clipboard în Spotlight (Tab pentru toggle), text, imagini, fișiere, preview + metadate, navigare ↑/↓ |
| Module manager | ✅ | catalog, manifest, download/verify/extract, progress, status |
| File watcher | ✅ | marchează indexul `stale` la schimbări în `roots`; nu pornește niciodată o tură singur |
| Embeddings CLIP | ✅ | `clip_text_encoder` + `clip_vision_encoder` + `clip_tokenizer` (ViT-B/32 int8), 512-d, un singur spațiu text–imagine |
| Indexare la cerere | ✅ | `index_files` pornește în fundal, `index_status` raportează fază/count/procent; singurul declanșator e chip-ul din Settings → General |
| Packaging | ✅ | script-uri DMG/MSI/DEB, binare daemon/CLI în `package-resources/` |
| SLM heuristic | ✅ | routing intenție + scaffold ONNX |
| Batching/downsampling indexare | ✅ | embeddings în sub-batches cu buget memorie, upsert batched LanceDB, procesare cloud în chunk-uri, downsampling vectorial |
| Teste Rust | ✅ | 63 passed cu feature-urile implicite (duckdb+onnx), 59 în config onnx-only; `tests/clip_space.rs` adaugă 5 teste pe greutăți reale, sărite când `MIRAGE_CLIP_MODELS` lipsește |
| Teste UI (jvmTest) | ✅ | 7 teste Compose pentru Spotlight, clipboard și Settings; aceleași scene exportă PNG în `build/ui-shots/` pentru comparat cu board-urile Penpot |

## Ce mai trebuie pentru local running complet

| Task | Prioritate | Note |
|------|------------|------|
| Config conectori din UI | ✅ finalizat | Tab Connectors în Settings cu add/edit/delete; salvare prin `update_connectors` |
| Footer index/module status indicator | ✅ finalizat | Cerc de progres + procent + cercuri icon surse cu toggle în footer |
| Refactor DuckDB/ONNX ca module descărcabile | ✅ finalizat | Feature-gate Cargo + catalog built-in + auto-ready + Settings Modules tab real |
| Testare packaging end-to-end | 🟢 scăzută | Necesită JDK 21 + Rust pe mediu potrivit; nu e blocant |
| Calibrare ranking cross-modal | 🟠 medie | intercalarea rezolvă vizibilitatea; mai lipsesc praguri absolute pe modalitate, ca un document slab să nu urce deasupra unei fotografii bune |
| Traseul worker/offload | 🟠 medie | tab-ul Servers descrie deja workerii; delta pull și garanția că nu pleacă credential-e trebuie verificate cap-coadă |
| Descărcări reale din Settings → Modules | 🟠 medie | OCR/Whisper/Sumarizare există în UI; catalogul implicit are acum doar module CLIP cu checksum verificat |

## 2026-08-31 — spațiu comun text–imagine real și UI-ul din Penpot în cod

**Ce s-a schimbat.** `ClipEmbedder` a înlocuit stub-ul pe bază de hash: aceleași greutăți
CLIP ViT-B/32 (int8, `Xenova/clip-vit-base-patch32`) pentru text și imagine, cu BPE real și
preprocesarea canonică a imaginii. Tabela LanceDB a trecut la 512-d cu coloane `modality` și
`caption`, reconstruită automat când dimensiunea salvată nu mai coincide cu a embedderului
încărcat. Indexarea se pornește doar din Settings → General; watcher-ul și modificările de
setări marchează `stale`. Clientul a fost adus la board-urile Penpot: bară de progres 4dp cu
count/total, chip „Start indexing"/„Re-index", Modulele citiesc `vision`/`semantic` din
`status.modules`, iar `excludedDirs` editate se trimit înapoi prin `update_indexing_settings`.

**Ce măsoară testele cu greutăți reale** (`src/daemon_next/tests/clip_space.rs`, 5 teste):

- text și imagine în același spațiu: cosine self ≈ 1.0, perechea corectă bate o pereche
  nelegată cu > 0.05;
- recuperare text → fotografie: **6/6** pe șase fotografii etichetate, cu marjă față de
  locul doi (ex. „a photo of a cat" → `cat.jpg` 0.288 vs. `car.jpg` 0.210);
- batched inference ≈ single-item inference (cosine > 0.98);
- pe un corpus mixt (6 fotografii + 6 documente), ranking-ul brut pe cosine lăsa
  **0 fotografii din 6** în fereașa vizibilă la toate interogările; după intercalare
  fotografia corectă apare pe poziția 2 la toate cele 6 interogări.

**Cap-coadă prin produs** (binar debug + `scripts/daemon-probe.py` pe socket, corpus cu
`node_modules/` exclus): `index_files` randează imediat, progresul merge
`Scanning files` → `Embedding local files, total 12` → `indexed 12, percent 100, stale false`;
căutările răspund în 12–18 ms, iar rezultatele alternează documente și fotografii.

**Reparații prinse de testele de integrare.** Suita `cargo test --tests` nu mai era
rulat complet de când `ipc_ping` cădea primul; odată reparat și el, testele au scos la
iveală trei defecte reale:

- daemonul nu-și crea socket-ul decât după ce înregistra watch-ul recursiv pe rădăcini;
  cu `roots` implicit (= home) depășea timeout-ul de pornire al clientului. Acum socket-ul
  ascultă primul, iar watch-ul se înregistrează după, pe un thread de blocking;
- căutarea direct pe vector (`search` cu `query_vector`) nu-și ajusta lățimea la dimensiunea
  tabelei, deci un client cu 384-d primea „failed to execute vector search" în loc de
  rezultate. Citirea face acum același resize ca scrierea;
- același fișier apărea de două ori în listă (un rând din indexul de nume + unul din cel
  semantic, cu scor negativ). Contopirea elimină duplicatul pe `id` și păstrează tier-ul mai bun.

Testele de integrare scriu acum un `daemon.yaml` propriu în directorul temporar, deci nu mai
citesc și nici nu mai suprascriu configurația dezvolțatorului din `target/debug/`.

**Cunoscut:** `tests/search_rpc.rs` e instabil când rulează paralel (patru daemoni care se
pornesc simultan pe un calculator deja ocupat); cu `--test-threads=1` trece.

**Reproducere.**

```sh
cd src/daemon_next
../../scripts/fetch-clip-models.sh test-models      # greutăți + tokenizer, cu sha256
../../scripts/fetch-test-images.py test-images      # 6 fotografii de pe Wikimedia Commons
MIRAGE_CLIP_MODELS=test-models MIRAGE_TEST_IMAGES=test-images cargo test --tests -- --test-threads=1
```

## Ce este exclus momentan

- **Onboarding / setup wizard:** va fi simplu și este exclus din sprintul curent. Descărcarea modelelor/motoarelor se va face direct din Settings → Modules sau prin indicatorul din footer.
- **Progres barul în Spotlight:** scos din fereastră în review-ul din 2026-08. Singurul loc pentru progres și pentru pornirea unei ture este Settings → General (bară 4dp cu count/total sau chip „Start indexing"); în fereastră rămâne doar filtrul de surse și key hints din footer.

## Design indicator indexare/module (footer)

Indicatorul de indexare/module se mută permanent în **footerul din stânga** al ferestrei Spotlight, sub forma unui cerc gol pe interior (doar outline). Stările vizuale:

- **Neindexat:** cerc gri, procent 0%.
- **Parțial:** outline galben care se completează progresiv; procent afișat lângă cerc.
- **Complet:** cerc verde plin; 100%.
- **Conectat:** în interiorul cercului parțial apar icon-uri suprapuse pentru sursele conectate (network volume, Dropbox, Google Drive, SMB etc.).

Click pe cerc deschide Settings la tabul de indexare/conectori. Sub cerc (sau direct în Settings dacă footerul este închis) apar modelele/motoarele cu switch-uri (unele active by default).

La pornire, în zona de sub bara de căutare (Spotlight) poate apărea un progress bar temporar pentru indexare; acesta dispare când indexarea e gata și poate fi închis manual.

## Cum se face packaging

```bash
# macOS DMG drag-and-drop
./scripts/package-macos.sh

# Windows MSI (pe Windows)
scripts\package-windows.bat

# Windows MSI cross-compilat de pe macOS/Linux (doar binare, MSI poate necesita Windows)
./scripts/package-windows-cross.sh

# Linux DEB
./scripts/package-linux.sh
```

Daemonul și CLI-ul sunt copiate în `src/client-kmp/package-resources/{macos,windows,linux}/` înainte de a rula Gradle. Aplicația le găsește prin `compose.application.resources.dir`.

## Blockere

- Buildul KMP nu a rulat pe acest Mac pentru că singurul JDK disponibil este Java 26, iar Gradle 8.14 nu-l suportă. Codul e scris pentru JDK 21 LTS.
- Packaging-ul nu a fost testat end-to-end; script-urile sunt pregătite, dar necesită JDK 21 + Rust.

## Commit-uri recente

- `38cf5ad` — footer indicator cu cerc de progres + filtre surse și RPC `list_connectors`.
- `3a84d97` — configurator conectori în Settings + fixuri compilare daemon (watcher, PathBuf, Arc connectors).
- `907ffb9` — packaging scripts, file watcher, explicit cloud download.
- `df0fa5f` — implementare conectori S3/R2, Dropbox, GDrive, SMB.
- `ce44181` — auto-spawn daemon în CLI și module status real în KMP.
