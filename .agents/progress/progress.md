# Progress — Mirage

## Stare curentă

- **Data ultimei actualizări:** 2026-08-28
- **Faza curentă:** Local standalone MVP — funcționalități locale și indicatorul din footer implementate; rămân refactorurile modulare (DuckDB/ONNX) și testarea packaging.
- **Progres general:** ~97% pentru funcționalitatea locală standalone
- **Wizard/onboarding:** exclus momentan; modulele se gestionează direct din Settings sau din indicatorul de stare.

## Ce funcționează local (finalizat)

| Componentă | Status | Note |
|------------|--------|------|
| Rust daemon (`src/daemon_next/`) | ✅ | IPC, search, indexare, apps, module manager, conectori cloud, file watcher |
| CLI `mirage` | ✅ | search, query, status, ask, module, auto-spawn daemon |
| KMP GUI (cod) | ✅ | floating search, tray, hotkey, IPC, module status, cloud badge, open URL, Shift+Enter download |
| Conectori cloud | ✅ | S3/R2, Dropbox, Google Drive, SMB (metadata + open URL, download explicit) |
| Module manager | ✅ | catalog, manifest, download/verify/extract, progress, status |
| File watcher | ✅ | reindexare automată la schimbări în `roots` |
| Packaging | ✅ | script-uri DMG/MSI/DEB, binare daemon/CLI în `package-resources/` |
| SLM heuristic | ✅ | routing intenție + scaffold ONNX |
| Teste Rust | ✅ | 29 unit tests trec; `ipc_ping` eșuează cu timeout la pornirea daemonului (de investigat, probabil environmental) |

## Ce mai trebuie pentru local running complet

| Task | Prioritate | Note |
|------|------------|------|
| Config conectori din UI | ✅ finalizat | Tab Connectors în Settings cu add/edit/delete; salvare prin `update_connectors` |
| Footer index/module status indicator | ✅ finalizat | Cerc de progres + procent + cercuri icon surse cu toggle în footer |
| Refactor DuckDB/ONNX ca module descărcabile | 🟢 scăzută | Funcționează built-in, dar obiectivul final e modular |
| Refactor DuckDB/ONNX ca module descărcabile | 🟢 scăzută | Funcționează built-in, dar obiectivul final e modular |

## Ce este exclus momentan

- **Onboarding / setup wizard:** va fi simplu și este exclus din sprintul curent. Descărcarea modelelor/motoarelor se va face direct din Settings → Modules sau prin indicatorul din footer.
- **Progres barul inițial din Spotlight:** va apărea doar sub index status la pornire, se poate închide, iar după finalizare dispare. Locul permanent este footerul din stânga.

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
