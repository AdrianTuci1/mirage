# CLI & MCP Server — Design Specification

## Scope

Acest document definește designul CLI-ului Mirage și al serverului MCP (Model Context Protocol). Se aplică task-urilor T10.1–T10.3 și T13.8. Nu include implementarea — doar structura, contractele, mapările RPC și deciziile de design.

## Context

- Daemonul Rust expune JSON-RPC 2.0 peste Unix Domain Socket (macOS/Linux) și Named Pipes (Windows).
- IPC client-ul existent este în `src/daemon/src/ipc/client.rs` și `src/daemon_next/src/ipc/client.rs`; folosește framing newline-delimited, request/response cu `id` fix la `1`.
- Protocolul și codurile de eroare sunt definite în `src/daemon/src/ipc/protocol.rs`.
- MCP serverul rulează în procesul CLI, se conectează la daemon prin IPC și folosește stdio ca transport MCP (conform ADR 010).

## 1. CLI Crate Structure

```
src/cli/
├── Cargo.toml
└── src/
    ├── main.rs      # entry point, argument parsing, subcommand dispatch
    ├── commands.rs  # implementarea fiecărei comenzi mirage <cmd>
    ├── mcp.rs       # server MCP peste stdio
    └── ipc.rs       # client IPC reutilizabil (opțional, poate rămâne în commands.rs la MVP)
```

### `Cargo.toml`

Crate separat, binar unic `mirage`. Dependențe:
- `tokio` — runtime async pentru conexiuni IPC.
- `clap` — parser CLI cu derive macros.
- `serde`, `serde_json` — serializare RPC și output JSON.
- `anyhow` — erori în CLI.
- `colored` sau `owo-colors` — output colorat pentru humans (opțional).
- `comfy-table` — tabele pentru humans (opțional).
- Pentru MCP: crate-ul oficial `rmcp` sau implementare manuală peste stdin/stdout cu `serde_json` (decizie de implementare, nu de design; MVP poate folosi parsing manual).

### `src/cli/src/main.rs`

- Parsează argumentele globale și subcomanda.
- Argument global obligatoriu/opțional: `--socket-path <PATH>`.
- Argument global opțional: `--json` (aplicabil pentru comenzile care produc output structurat; poate fi suprascris per comandă).
- Descoperă calea implicită a socket-ului relativ la directorul părinte al executabilului (vezi secțiunea 3).
- Inițializează `IpcClient` cu socket path și apelează handler-ul subcomenzii.

### `src/cli/src/commands.rs`

Aici locuiesc handler-ele pentru fiecare comandă. Toate folosesc o funcție comună pentru a trimite RPC și a interpreta răspunsul.

### `src/cli/src/mcp.rs`

Implementarea serverului MCP. Rulează în `mirage mcp serve`. Citește mesaje JSON-RPC MCP de la stdin, trimite request-uri către daemon prin IPC, răspunde către stdout. Nu include parsing CLI în afară de `--socket-path`.

## 2. Comenzi și Argumente

### 2.1 Global options

```
USAGE:
    mirage [OPTIONS] <COMMAND>

OPTIONS:
    --socket-path <PATH>    Calea explicită către socket/pipe. Dacă lipsește,
                            se derivează din directorul executabilului.
    -h, --help              Ajutor
    -V, --version           Versiune
```

### 2.2 `mirage search`

```
USAGE:
    mirage search [OPTIONS] <QUERY>

ARGS:
    <QUERY>    Textul căutat

OPTIONS:
    -k, --top-k <N>         Număr maxim de rezultate [default: 10]
    -b, --hybrid            Activează căutarea hibridă (vector + BM25 + SQL)
    -j, --json              Output JSON în loc de tabel
```

Mapare RPC:
- `method`: `search`
- `params`: `{"query": "<QUERY>", "top_k": N, "hybrid": true|false}`

Output uman: tabel cu coloane `score`, `path`, `source_type`.
Output JSON: array de obiecte `SearchResult`:
```json
[
  {"id": "...", "relative_path": "...", "score": 0.95, "source_type": "local"}
]
```

### 2.3 `mirage query`

```
USAGE:
    mirage query <SQL>

ARGS:
    <SQL>    Interogare SQL DuckDB

OPTIONS:
    -j, --json    Output JSON raw
```

Mapare RPC:
- `method`: `query`
- `params`: `{"sql": "<SQL>"}`

Output uman: tabel cu header-ele din primul rând și toate rândurile, sau mesaj de confirmare pentru comenzi DDL/DML fără rezultat.
Output JSON: obiect `{"columns": [...], "rows": [...]}`.

### 2.4 `mirage status`

```
USAGE:
    mirage status [OPTIONS]

OPTIONS:
    -j, --json    Output JSON
```

Mapare RPC:
- `method`: `status`
- `params`: `null`

Output uman: status daemon, module active, versiune, socket path, eventuale module lipsă.
Output JSON: obiectul `status` returnat de daemon.

### 2.5 `mirage index`

```
USAGE:
    mirage index [OPTIONS] <PATH>

ARGS:
    <PATH>    Calea către fișier sau director de indexat

OPTIONS:
    -s, --source-type <TYPE>    local | nas | dropbox | s3 | gdrive [default: local]
    -r, --recursive             Indexează recursiv directoarele (la nivel CLI, daemonul decide logica)
```

Mapare RPC:
- `method`: `index`
- `params`: `{"path": "<PATH>", "source_type": "<TYPE>"}`

Output uman: `Indexed N items from <PATH>` sau progress incremental.
Output JSON: `{"indexed": N, "path": "...", "source_type": "..."}`.

### 2.6 `mirage modules`

```
USAGE:
    mirage modules [OPTIONS]

OPTIONS:
    -j, --json    Output JSON
```

Mapare RPC:
- `method`: `module_status` (sau `status` dacă daemonul include listă în `status`)
- `params`: `null`

Output uman: tabel cu `module_id`, `status` (installed | available | downloading | missing), `size`, `version`.
Output JSON: array de obiecte module.

### 2.7 `mirage download`

```
USAGE:
    mirage download <MODULE>

ARGS:
    <MODULE>    ID-ul modulului: duckdb | onnx | slm_sql | ...

OPTIONS:
    --no-wait    Nu aștepta finalizarea descărcării; returnează imediat job ID
```

Mapare RPC:
- `method`: `download_module`
- `params`: `{"module_id": "<MODULE>"}`

Output uman: progres periodic citit din evenimente (dacă daemonul suportă) sau mesaj final `Module <MODULE> installed`.
Output JSON: `{"module_id": "...", "status": "installed|downloading|failed", "job_id": "..."}`.

### 2.8 `mirage ask`

```
USAGE:
    mirage ask <QUESTION>

ARGS:
    <QUESTION>    Întrebare în limbaj natural (multilingual)

OPTIONS:
    -j, --json    Output JSON
```

Mapare RPC:
- `method`: `ask`
- `params`: `{"question": "<QUESTION>"}`

Output uman:
- Dacă intenția este `semantic_search`: listă de rezultate.
- Dacă intenția este `sql_query`: răspuns în limbaj natural (nu SQL brut).

Output JSON:
```json
{
  "question": "...",
  "type": "semantic_search | sql_query",
  "results": [...],
  "natural_language_answer": "..."
}
```

### 2.9 `mirage mcp serve`

```
USAGE:
    mirage mcp serve [OPTIONS]

OPTIONS:
    --socket-path <PATH>    Override socket path
```

Pornește serverul MCP peste stdio. Nu acceptă alți parametri. Vezi secțiunea 6 pentru detalii MCP.

## 3. Descoperirea Căii Socket-ului

CLI derivează calea implicită astfel:

1. Determină directorul părinte al executabilului curent: `std::env::current_exe().parent()`.
2. Rezolvă relativ la acel director: `../daemon.yaml` sau caută direct fișierul socket/pipe:
   - Unix: `<parent>/mirage.sock` sau `<parent>/mirage.socket`
   - Windows: numele pipe-ului `mirage` (cu prefixul `\\.\pipe\` aplicat în client).
3. Dacă există `daemon.yaml` în directorul părinte sau în `<parent>/..`, citește `socket_path` din config.
4. Ordinea de precedență:
   1. `--socket-path <PATH>` (cea mai mare prioritate)
   2. `daemon.yaml::socket_path` dacă fișierul există
   3. default relativ la executabil

Rationale: CLI și daemonul sunt distribuite împreună; la dezvoltare pot rula din directoare diferite, dar în instalare stau în același bundle.

## 4. IPC Client Abstraction

CLI reutilizează logica deja existentă din daemon. Pentru a nu duplica protocolul, sunt două opțiuni:

### Opțiunea A: Copy protocol/client (MVP recomandat)
Copiază `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError` și funcția `call` în `src/cli/src/ipc.rs`. Avantaje: crate independent, build rapid, fără deps suplimentare.

### Opțiunea B: Shared crate
Extrage IPC protocol într-un crate `mirage-ipc` în `src/ipc/` și îl includ atât daemonul cât și CLI. Avantaje: single source of truth. Dezavantaje: refactor mai mare.

### Funcție comună propusă (Opțiunea A)

```rust
pub async fn ipc_call(socket_path: &Path, method: &str, params: Option<Value>) -> Result<Value, CliError>;
```

Comportament:
1. Conectare la socket/pipe.
2. Serializează `JsonRpcRequest` cu `jsonrpc: "2.0"`, `id: 1`.
3. Trimite mesajul urmat de `\n`.
4. Citește o linie.
5. Deserializează `JsonRpcResponse`.
6. Dacă `error` este `Some`, returnează `CliError::Rpc { code, message, data }`.
7. Returnează `result.unwrap_or(Value::Null)`.

## 5. Output Formatting

### Modul uman
- Tabele cu `comfy-table` pentru liste și rezultate SQL.
- Mesaje scurte, colorate, fără JSON.
- Erori prefixate cu `error:`.

### Modul JSON (`--json`)
- Tot output-ul valid este un singur obiect JSON sau array JSON.
- Succes: obiectul `{"ok": true, "data": <result>}` sau direct rezultatul, la alegerea implementării (se recomandă wrapper `{"ok": true, "data": ...}` pentru consistență).
- Eroare: `{"ok": false, "error": {"kind": "...", "message": "..."}}`.

### Structured errors
Fiecare eroare are un `kind`:
- `daemon_unavailable` — daemonul nu rulează sau socket-ul nu există.
- `method_not_found` — daemonul nu cunoaște metoda (de ex. MCP tool mapat la metodă inexistentă).
- `missing_module` — modulul necesar nu este instalat; include `module_id` în `data`.
- `invalid_params` — parametri CLI sau RPC invalidi.
- `network` — eroare de descărcare (în `download`).
- `internal` — eroare internă daemon/CLI.

## 6. MCP Server

### Transport
- stdio: citește linii JSON-RPC de la stdin, scrie la stdout.
- Fiecare mesaj MCP este un obiect JSON-RPC 2.0, separate prin newline.

### Capabilities anunțate
Serverul răspunde la `initialize` cu:
- `tools`: `{ listChanged: false }`
- `resources`: `{ listChanged: false, subscribe: false }`
- `prompts`: `{ listChanged: false }`
- `logging`: `{}`

### Tools

| Nume MCP        | Parametri                              | Metodă IPC          | Descriere                              |
|-----------------|----------------------------------------|---------------------|----------------------------------------|
| `search`        | `query`, `top_k`, `hybrid`             | `search`            | Căutare semantică/hibridă              |
| `query`         | `sql`                                  | `query`             | Execută SQL DuckDB                     |
| `index_path`    | `path`, `source_type`                  | `index`             | Indexează un fișier sau director       |
| `status`        | —                                      | `status`            | Stare daemon și module                 |
| `ask`           | `question`                             | `ask`               | Răspunde la întrebări în limbaj natural (routing semantic/SQL) |
| `download_module`| `module_id`                            | `download_module`   | Declanșează descărcarea unui modul     |

Exemplu mapping pentru `search`:
```json
{
  "name": "search",
  "description": "Search indexed local files using semantic and/or hybrid search.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {"type": "string"},
      "top_k": {"type": "integer", "default": 10},
      "hybrid": {"type": "boolean", "default": false}
    },
    "required": ["query"]
  }
}
```

La apel, serverul MCP construiește `JsonRpcRequest { method: "search", params: { ... }, id: 1 }`, îl trimite prin IPC, apoi returnează `result` sau `error` ca răspuns MCP `tools/call`.

### Resources

Resursele sunt read-only și reprezintă fișierele indexate.

- `uri`: `mirage://files/<source_type>/<relative_path>` (URL-encoded).
- Conținut: metadata JSON (path, source_type, updated_at, version) — conținutul brut al fișierului nu este expus prin MCP pentru a evita leakage de date sensibile.
- Listarea resurselor se face prin RPC `search` cu `query: ""` și `top_k: 1000` sau printr-o metodă dedicată `list_indexed_files` dacă daemonul o adaugă ulterior.

### Prompts

Serverul oferă prompt-uri de exemplu pentru agenți:
- `search-examples`: "Caută facturi din 2024", "Găsește contracte cu vendor X".
- `sql-examples`: "Câte documente am indexat?", "Care sunt fișierele modificate săptămâna trecută?".

## 7. Error Handling

### Daemon not running
Dacă `UnixStream::connect` sau `ClientOptions::open` eșuează:
- Uman: `error: daemon not running at <path>; start it with ...`
- JSON: `{"ok": false, "error": {"kind": "daemon_unavailable", "message": "...", "data": {"socket_path": "..."}}}`
- MCP: returnează eroare tool `internal_error` cu mesajul corespunzător.

### Method not found
Dacă răspunsul RPC conține `error.code == -32601`:
- Uman: `error: daemon method '<method>' not found; daemon may need upgrade`
- JSON: `{"ok": false, "error": {"kind": "method_not_found", "message": "..."}}`
- MCP: `internal_error` cu sugestie de upgrade.

### Missing module
Dacă daemonul returnează `error.data.module_id`:
- Uman: `error: module '<id>' is missing; run 'mirage download <id>'`
- JSON: `{"ok": false, "error": {"kind": "missing_module", "message": "...", "data": {"module_id": "..."}}}`
- MCP: returnează eroare tool cu același mesaj.

### Network issues
În `download`, dacă daemonul semnalează eroare de descărcare:
- Uman: `error: failed to download module '<id>': <message>`
- JSON: `{"ok": false, "error": {"kind": "network", "message": "...", "data": {"module_id": "..."}}}`

### MCP protocol errors
- Parse error: răspuns JSON-RPC cu `error.code = -32700`.
- Invalid params: răspuns cu `error.code = -32602`.

## 8. Integration with Modular Download Manager (T13.x)

Comanda `mirage download <MODULE>` este punctul de intrare CLI pentru Modular Download Manager.

Flux:
1. CLI parsează `<MODULE>` (ex. `duckdb`, `onnx`, `slm_sql`).
2. Trimite RPC `download_module` către daemon.
3. Daemonul validează `module_id` împotriva catalogului local, rezolvă dependențele (ex. `slm_sql` depinde de `onnx`), începe descărcarea HTTPS.
4. Fără `--no-wait`:
   - CLI așteaptă un eveniment final de finalizare/eroare prin IPC (dacă daemonul implementează notificări) sau face polling `module_status` până când statusul devine `installed`/`failed`.
   - Timeout implicit: 10 minute.
5. Cu `--no-wait`:
   - CLI returnează imediat `job_id` și statusul inițial.

Comanda `mirage modules` listează modulele din catalog (`available`, `installed`, `missing`, `downloading`) folosind RPC `module_status`.

Comanda `mirage ask` necesită modulele `onnx_runtime` și `slm_nl_router`. Dacă daemonul returnează `missing_module`, CLI afișează mesajul și sugestia de download.

Comanda `mirage query` necesită modulul `duckdb`. Dacă lipsește, eroare `missing_module` cu `module_id: "duckdb"`.

## 9. Decizii și Riscuri

| Decizie | Rationale |
|---------|-----------|
| Crate CLI separat de daemon | Build rapid, binar mic, poate fi distribuit independent. |
| Socket path relativ la executabil | Bundle self-contained; nu poluează `~/.mirage`. |
| MCP peste stdio | Standard MCP; fără porturi, fără configurare de rețea. |
| MCP proxy către daemon | Logica de business rămâne în daemon; CLI rămâne thin client. |
| `--json` pentru toate comenzile | Scriptabil și testabil. |
| Resources read-only + metadata | Securitate: evită expunerea conținutului brut al fișierelor către agenți. |

Riscuri:
- Crește complexitatea CLI (arg parsing + MCP + IPC). Mitigare: se implementează incrementally, comenzile simple primele.
- `rmcp` sau parsing manual MCP poate fi instabil. Mitigare: parsing manual newline-delimited pentru MVP.
- Windows Named Pipes pot avea latency mai mare. Mitigare: teste pe Windows în CI.

## 10. Link-uri

- ADR 010: `.agents/decisions/adr/010-mcp-support.md`
- Technical spec: `.agents/specs/technical-spec.md`
- IPC protocol: `src/daemon/src/ipc/protocol.rs`
- IPC client: `src/daemon/src/ipc/client.rs`
