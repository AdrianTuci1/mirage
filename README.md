# Mirage

Mirage este un motor de căutare semantică **local-first** cu arhitectură de daemon. Un proces Rust rulează în fundal și oferă căutare, indexare, analitică SQL și ML local. GUI-ul desktop și CLI-ul sunt clienți care comunică cu daemonul prin IPC.

Toate datele locale și modelele descărcate se păstrează în folderul aplicației. La dezinstalare, nimic nu rămâne în `~/.mirage` sau `Documents`.

## Arhitectură

```
                          ┌───────────────────────────┐
                          │   MIRAGE GUI (KMP App)    │
                          └─────────────┬─────────────┘
                                        │
                                        ▼ (IPC / Socket)
┌───────────────────────────────────────────────────────────────────────────────┐
│                           MIRAGE DAEMON (Rust Background)                       │
│                                                                               │
│  • Vector & Text Index  (LanceDB / Tantivy)                                   │
│  • Tabular & SQL Engine (DuckDB / Parquet — descărcabil la cerere)            │
│  • Embedded ML          (ONNX / Rust SIMD — descărcabil la cerere)           │
│  • Sync Worker          (Local Stream / Remote Cloud Sync)                    │
└───────────────────────────────▲───────────────────────────────────────────────┘
                                │
                                ▼ (IPC / Socket)
                          ┌───────────────────────────┐
                          │  CLI & MCP Clients          │
                          └─────────────┬─────────────┘
                                        │
                                        ▼ (Stdio / Socket)
                          ┌───────────────────────────┐
                          │  TERMINAL COMMANDS          │
                          └───────────────────────────┘
```

## Module

- `src/daemon/` — Core Daemon Rust (în planificare).
- `src/cli/` — CLI Rust (în planificare).
- `src/remote-indexer/` — Worker Python pentru procesare remote (Docker).
- `src/client-kmp/` — Aplicație desktop Kotlin Multiplatform + Compose.
- `src/shared/` — Cod și scripturi comune.

## Monetizare

| Tier | Infrastructură | Admin | Cost |
|------|----------------|-------|------|
| Community Standalone | Local | Aplicație desktop | Gratuit |
| Community Self-Hosted | Propriu VM/Docker | Admin Web Console în worker | Gratuit |
| Managed Cloud | Cluster orchestrat | Dashboard SaaS | Abonament |

## Documentație

Toate specificațiile, deciziile și planul de execuție sunt în `.agents/`.

- `.agents/specs/technical-spec.md`
- `.agents/specs/pricing-monetization.md`
- `.agents/execution-graph/project-graph.json`
- `.agents/architecture/system-overview.md`
- `.agents/decisions/adr/`

## UI / Design System

Designul aplicației este definit în:

- `docs/ui-design-system.md` — tokens de culoare/spațiere/tipografie, structura ferestrei de căutare, fereastra de settings, comportamente.

## Stare

Proiect în dezvoltare activă. Faza curentă: M13 modular download manager + IPC implementate; urmează SLM, refactor module și CLI/GUI.
