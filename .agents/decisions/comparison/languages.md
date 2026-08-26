# Comparare Limbaje — Mirage

## Context

Trebuie să alegem limbajul pentru Remote Indexer și pentru clientul desktop.

## Remote Indexer

| Criteriu | Python 3.11 | Rust |
|----------|-------------|------|
| Ecosistem ML | **Excelent** | Limitat |
| LanceDB support | **Excelent** | Bun |
| Docker image size | Medie | **Mică** |
| Concurrency | GIL | **Native** |
| Timp de dezvoltare | **Rapid** | Lent |
| Memory safety | Runtime | **Compile-time** |

## Client Desktop

| Criteriu | Kotlin Multiplatform | Electron | Tauri |
|----------|----------------------|----------|-------|
| UI framework | Compose | HTML/JS | WebView |
| Performanță nativă | **Da** | Nu | Da |
| Single codebase | **Da** | Da | Da |
| LanceDB integration | Via JVM/JNI | Via Node | Via Rust |
| Look & feel nativ | **Da** | Web-like | Web-like |

## Recomandare

- **Remote Indexer**: Python 3.11 pentru MVP.
- **Client**: Kotlin Multiplatform cu Compose Desktop.
