# ADR 008: Core Daemon + IPC Architecture

## Status

Accepted

## Context

Inițial, Mirage era conceput ca o aplicație KMP desktop care rulează motorul de căutare în procesul JVM. Pe măsură ce produsul a evoluat, au apărut cerințe noi:

- CLI independent pentru terminal.
- Integrare cu agenți AI prin MCP.
- Rularea continuă în fundal, chiar și când GUI-ul este închis.
- Performanță maximă pentru embedding, vector search și DuckDB.

Rularea acestor funcții în JVM nu este optimă pentru SIMD, acces nativ și footprint redus.

## Decizie

Implementăm un **Core Daemon în Rust** care rulează ca proces de fundal. GUI-ul KMP, CLI-ul și clienții MCP devin clienți IPC.

- **Daemon**: Rust, cu LanceDB, DuckDB, ONNX Runtime, IPC socket.
- **IPC**: Unix Domain Sockets pe Linux/macOS, Named Pipes pe Windows.
- **Protocol**: JSON-RPC 2.0 peste IPC.
- **GUI**: KMP/Compose Desktop, doar client vizual.
- **CLI**: Rust/Go, binar lightweight.
- **MCP**: `mirage mcp serve` peste stdio.

## Consecințe

- GUI-ul devine mai simplu: doar UI și IPC client.
- CLI și MCP primesc aceeași capacitate ca GUI.
- Daemonul poate rula la startup și rămâne în tray.
- Securitate mai bună: IPC este protejat de filesystem, nu expus pe TCP.
- Crește complexitatea: trebuie implementat daemon Rust + IPC + clienți.
- KMP clientul local de vector store devine temporar sau poate fi înlocuit cu client IPC.

## Note

Această decizie înlocuiește arhitectura anterioară în care clientul KMP rula interogări local.
