# Progress — Mirage

## Stare curentă

- **Data ultimei actualizări:** 2026-08-28
- **Faza curentă:** Local standalone MVP aproape complet
- **Progres general:** ~85% pentru funcționalitatea locală standalone

## Ce funcționează local (finalizat)

| Componentă | Status | Note |
|------------|--------|------|
| Rust daemon (`src/daemon_next/`) | ✅ | IPC, search, indexare, apps, module manager, conectori cloud |
| CLI `mirage` | ✅ | search, query, status, ask, module, auto-spawn daemon |
| KMP GUI (cod) | ✅ | floating search, tray, hotkey, IPC, module status, cloud badge, open URL |
| Conectori cloud | ✅ | S3/R2, Dropbox, Google Drive, SMB (metadata + open URL, fără descărcare automată) |
| Module manager | ✅ | catalog, manifest, download/verify/extract, progress, status |
| SLM heuristic | ✅ | routing intenție + scaffold ONNX |
| Teste Rust | ✅ | unit + integration trec |

## Ce mai trebuie pentru local running complet

| Task | Prioritate | Note |
|------|------------|------|
| Packaging & installer (DMG/MSI/DEB) | 🔴 ridicată | Bundling daemon + GUI + CLI; pe mac drag-and-drop, pe Windows MSI |
| File watcher auto-reindex | 🔴 ridicată | Reindexare automată la schimbări fișiere locale |
| Setup wizard / onboarding | 🟡 medie | Descărcare modele/ motoare la prima pornire |
| Config conectori din UI | 🟡 medie | Form-uri pentru S3/Dropbox/GDrive/SMB în Settings |
| Download explicit cloud | 🟡 medie | Shift+Enter salvează în Downloads real (momentan placeholder) |
| Refactor DuckDB/ONNX ca module descărcabile | 🟢 scăzută | Funcționează built-in, dar obiectivul final e modular |

## Blockere

- Buildul KMP nu a rulat pe acest Mac pentru că singurul JDK disponibil este Java 26, iar Gradle 8.14 nu-l suportă. Codul e scris pentru JDK 21 LTS.
- Packaging-ul nu a fost încă implementat.

## Commit-uri recente

- `df0fa5f` — implementare conectori S3/R2, Dropbox, GDrive, SMB.
- `ce44181` — auto-spawn daemon în CLI și module status real în KMP.
