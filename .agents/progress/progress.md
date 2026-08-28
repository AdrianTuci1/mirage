# Progress — Mirage

## Stare curentă

- **Data ultimei actualizări:** 2026-08-28
- **Faza curentă:** Local standalone MVP — funcționalități locale implementate; rămân configuratorul de conectori și setup wizard.
- **Progres general:** ~92% pentru funcționalitatea locală standalone

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
| Teste Rust | ✅ | unit + integration trec |

## Ce mai trebuie pentru local running complet

| Task | Prioritate | Note |
|------|------------|------|
| Config conectori din UI | 🔴 ridicată | Form-uri pentru S3/Dropbox/GDrive/SMB în Settings; momentan se editează `daemon.yaml` |
| Setup wizard / onboarding | 🟡 medie | Descărcare modele/motoare la prima pornire |
| Refactor DuckDB/ONNX ca module descărcabile | 🟢 scăzută | Funcționează built-in, dar obiectivul final e modular |

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

- `907ffb9` — packaging scripts, file watcher, explicit cloud download.
- `df0fa5f` — implementare conectori S3/R2, Dropbox, GDrive, SMB.
- `ce44181` — auto-spawn daemon în CLI și module status real în KMP.
