# SLM Natural Language Router & SQL Generator — Design Specification

## Scop

Această specificație definește un **small language model (SLM)** descărcabil la cerere, care:

- primește o întrebare în limbaj natural (poate fi în mai multe limbi);
- **decide intenția**: căutare semantică (`semantic_search`) sau interogare SQL (`sql_query`);
- pentru `semantic_search` — returnează parametrii pentru căutarea semantică (textul de căutare), iar daemonul execută `search()` și returnează **multiple rezultate**;
- pentru `sql_query` — generează SQL DuckDB, îl execută prin `query()`, apoi returnează **doar concluzia în limbaj natural**, nu SQL-ul brut;
- rulează direct în daemon prin ONNX Runtime, fără `llama.cpp` / GGUF.

Modelul nu este inclus în binarul de bază; se descarcă ca modul `slm_nl_router` la cerere.

Documentul este design-only; nu include cod compilabil.

## 1. Alegerea modelului

### 1.1 Cerințe actualizate

Pentru faza 1, modelul trebuie să fie:

- **Multilingv**: să înțeleagă întrebări în mai multe limbi și să poată răspunde în aceeași limbă.
- **Router de intenție**: să decidă între `semantic_search` și `sql_query`.
- **SQL generator**: să producă SQL DuckDB valid când intenția este `sql_query`.
- **Sumarizator**: să transforme rezultatul SQL într-un răspuns în limbaj natural.
- **Descărcabil**: nu vine în binar; se descarcă ca modul opțional.

### 1.2 Opțiuni evaluate

| Model | Parametri | Dimensiune estimată ONNX INT8 | Avantaje | Dezavantaje |
|---|---|---|---|---|
| `bigscience/mt0-small` | ~300 M | ~120–160 MB | Multilingv nativ (mT5 + multitask tuning), bun pentru instructiuni și routing | Mai mare decât flan-t5-small; text-to-SQL mai puțin specializat |
| `google/flan-t5-small` | 80 M | ~60–90 MB | Foarte mic, rapid pe CPU, bun cu prompt-uri few-shot | **Predominant engleză**; performanță slabă pentru alte limbi și pentru summarizare multilingvă |
| `google/flan-t5-base` | 250 M | ~200–250 MB | Calitate mai bună, mai bun decât small la routing | Tot englez-centric; dimensiune mare pentru un model non-multilingv |
| `Qwen/Qwen2-0.5B-Instruct` | ~0.5 B | ~250–350 MB (INT8) | Foarte bun multilingv, chat/instruct, bun pentru routing și NL | Decoder-only; necesită gestiune KV cache; mai mare decât mt0-small |
| `defog/sqlcoder-2b` / distilat | ~2 B | ~450–500 MB | Specializat pentru text-to-SQL | Multilingv slab, prea mare pentru faza 1 |

### 1.3 Recomandare faza 1

**Default:** `bigscience/mt0-small` cu export ONNX și quantizare INT8. Este cel mai mic model care acoperă în mod rezonabil toate cele trei cerințe (multilingv + routing + text-to-SQL + summarizare).

- Dimensiune descărcabilă: **~120–160 MB**.
- Licență: permisivă pentru cercetare/comercial (verifică licența BigScience T0).
- Funcționează ca seq2seq, deci poate fi antrenat/promptat pentru toate cele 3 task-uri cu prompt-uri diferite.

**Variantă opțională mai mică:** `google/flan-t5-small` pentru utilizatori care lucrează doar în engleză și vor o descărcare sub 100 MB.

**Variantă premium:** `Qwen/Qwen2-0.5B-Instruct` INT8 pentru utilizatori care acceptă ~300 MB și vor cea mai bună calitate multilingvă.

Modelele > 500 MB (sqlcoder-7b etc.) nu sunt incluse în catalogul default.

## 2. Pachetul `slm_nl_router`

Modelul este livrat ca modul descărcabil, conform `.agents/specs/modular-download-manager.md` §3.4.

```json
{
  "id": "slm_nl_router",
  "name": "SLM Natural Language Router",
  "version": "1.0.0",
  "description": "Small ONNX model that translates natural language questions into DuckDB SQL.",
  "kind": "model",
  "license": "Apache-2.0",
  "is_optional": true,
  "dependencies": ["onnx_runtime", "duckdb"],
  "platforms": {
    "universal": {
      "url": "https://cdn.mirage.ai/modules/slm-sql-model/1.0.0/slm-sql-model-1.0.0-universal.tar.gz",
      "size": 104857600,
      "checksum": "...",
      "archive_format": "tar.gz",
      "files": [
        {"relative_path": "model.onnx", "sha256": "...", "executable": false, "required": true},
        {"relative_path": "tokenizer.json", "sha256": "...", "executable": false, "required": true},
        {"relative_path": "generation_config.json", "sha256": "...", "executable": false, "required": false}
      ]
    }
  }
}
```

Layout după extracție:

```
<app-bundle>/models/slm_sql_model/1.0.0/
  model.onnx
  tokenizer.json
  generation_config.json   (opțional)
```

### 2.1 Dependințe

- `onnx_runtime` — runtime nativ încărcat din `<app-bundle>/downloads/onnx_runtime/<version>/lib/`.
- `duckdb` — folosit pentru a obține schema tabelelor și, ulterior, pentru a executa SQL generat.
- opțional `text_embedding_model` — dacă modelul decide `semantic_search`, daemonul poate folosi același embedder text pentru a produce vectorul de căutare.

### 2.2 Comportament

Modelul este un **router + generator + summarizator**. Nu returnează SQL brut utilizatorului final. Pentru `sql_query`, daemonul generează SQL, îl execută, iar apoi folosește același model pentru a sumariza rezultatul în limbaj natural.

Exemple:

- Întrebare: *"câte documente am indexat?"*
  - Intenție: `sql_query`
  - Rezultat final: *"Ai indexat 42 de documente."*

- Întrebare: *"contracte cu Adrian"*
  - Intenție: `semantic_search`
  - Rezultat final: *"Am găsit 10 rezultate. Iată primele: ..."* (lista de rezultate din `search()`).

- Întrebare: *"documents about Adrian"*
  - Intenție: `semantic_search` (limba nu afectuează ruta, doar promptul intern este adaptat).
  - Rezultat final: lista de rezultate semantice.

## 3. Template-uri de prompt

Modelul este folosit pentru trei task-uri, cu prompturi diferite. Toate sunt șiruri seq2seq și returnează text generat.

### 3.1 Task A — Clasificare intenție (`intent_classification`)

Determină dacă utilizatorul vrea o căutare semantică sau o interogare SQL.

```text
You are a query intent classifier. Choose exactly one of: semantic_search | sql_query.

A semantic_search query asks for documents, files, content, or similarity.
A sql_query asks for counts, aggregations, filtering, dates, or tabular analytics.

Question: {user_question}
Intent:
```

Exemplu:

```text
Question: câte documente am indexat?
Intent: sql_query

Question: contracte cu Adrian
Intent: semantic_search

Question: files modified this week
Intent: sql_query
```

### 3.2 Task B — Generare SQL (`sql_generation`)

Se execută doar când intenția este `sql_query`.

```text
You are a DuckDB SQL assistant. Generate only one SQL query between the markers.

-- Schema
{schema_context}

-- Examples
Question: {example_question_1}
SQL: ```sql
{example_sql_1}
```

Question: {example_question_2}
SQL: ```sql
{example_sql_2}
```

-- Question
Question: {user_question}
SQL:
```

### 3.3 Task C — Sumarizare rezultat (`result_summarization`)

Se execută după ce SQL-ul a fost rulat. Modelul primește un fragment al rezultatului (max ~5 rânduri sau count) și produce un răspuns în limbaj natural.

```text
You are a helpful assistant. Summarize the query result in the same language as the user's question.

User question: {user_question}
SQL result (first rows):
{result_preview}

Answer in natural language:
```

### 3.4 Limite

- Schema context este trunchat la numărul maxim de tokeni acceptați de model (de ex. 512 tokeni).
- Dacă utilizatorul specifică `context.tables`, se includ doar acele tabele; altfel se includ toate tabelele din cache, până la limită.
- Rezultatele SQL sunt trunchate pentru sumarizare (max 5 rânduri + count total) pentru a nu depăși contextul modelului.
- Limbajul de răspuns este determinat de limbajul întrebării; modelul multilingv (mt0) menține limba întrebării.

## 4. Tokenizare

### 4.1 Fișiere tokenizer

Pachetul include fișierul `tokenizer.json` în format Hugging Face Fast Tokenizers. Acesta conține vocabularul, pre-tokenizatorul, normalizatorul și regulile speciale (BOS/EOS/PAD/UNK).

Dacă modelul ales vine doar cu `tokenizer.model` (SentencePiece), catalogul va include și `spiece.model`, iar implementarea va încerca fallback la crate-ul `sentencepiece`. Formatul preferat rămâne `tokenizer.json` pentru consistență cu modelul de embeddings.

### 4.2 Încărcare

Tokenizerul este încărcat o dată cu sesiunea ONNX, la activarea modulului:

```rust
// pseudocod
let tokenizer = Tokenizer::from_file(models_dir.join("tokenizer.json"))?;
let session = Session::builder()
    .with_optimization_level(GraphOptimizationLevel::Level3)?
    .commit_from_file(models_dir.join("model.onnx"))?;
```

### 4.3 Encode / decode

- **Encode**: `tokenizer.encode(prompt, true)` produce `input_ids` și `attention_mask` ca vectori `i64`.
- **Decode**: tokenii generați sunt convertiți în string cu `tokenizer.decode(token_ids, true)`; apoi se elimină tokenurile speciale (`<pad>`, `</s>`, etc.).

## 5. Fluxul de inferență ONNX

### 5.1 Pași

1. **Construire prompt**: se combină schema context, exemplele și întrebarea.
2. **Tokenizare**: promptul devine tensori `input_ids` și `attention_mask` de formă `[1, seq_len]`.
3. **Generare autoregresivă**:
   - Pentru modele encoder-decoder (T5): se rulează encoderul o singură dată, apoi se buclă decoderul până la EOS sau `max_new_tokens`.
   - Pentru modele decoder-only (SQLCoder): se buclă modelul cu `past_key_values` (dacă modelul ONNX exportă cache).
   - Strategie de decodare: **greedy** ca default; opțional beam search = 1 (greedy) pentru MVP.
4. **Extragere SQL**: textul generat este scanat pentru markerii ` ```sql ` / ` ``` ` sau `SQL:` / `ENDSQL`.
5. **Post-procesare**: se curăță whitespace și markdown fences.

### 5.2 Exemplu de pseudocod (nu se compilează)

```rust
fn generate_sql(&self, question: &str, schema: &str) -> Result<SqlOutput> {
    let prompt = build_prompt(schema, question);
    let encoding = self.tokenizer.encode(prompt, true)?;
    let input_ids = encoding.get_ids();
    let attention_mask = encoding.get_attention_mask();

    let mut generated = vec![decoder_start_id];
    let max_new_tokens = self.config.max_new_tokens;

    for _ in 0..max_new_tokens {
        let outputs = self.session.run(build_inputs(&input_ids, &attention_mask, &generated))?;
        let next_token = argmax(logits); // greedy
        if next_token == eos_id {
            break;
        }
        generated.push(next_token);
    }

    let raw = self.tokenizer.decode(&generated, true)?;
    extract_sql(&raw)
}
```

### 5.3 Optimizări

- `GraphOptimizationLevel::Level3` pe ONNX Runtime.
- Quantizare INT8 pentru weights (dynamic quantization) pentru a reduce dimensiunea și a crește viteza pe CPU.
- Număr fix de thread-uri controlat prin `config.intra_op_num_threads` (default 4 pe desktop, 2 pe laptopuri cu baterie).
- Se poate folosi `Session::run` sincron; modelul fiind mic, nu este obligatoriu async în primă fază.

## 6. Metodă IPC nouă: `ask(question)`

Înlocuim vechea metodă `generate_sql` cu o metodă mai generală:

```json
{
  "jsonrpc": "2.0",
  "method": "ask",
  "params": {
    "question": "câte documente am indexat?",
    "context": { "tables": ["documents"] }
  },
  "id": 1
}
```

Răspunsuri posibile:

### `semantic_search`

```json
{
  "jsonrpc": "2.0",
  "result": {
    "type": "semantic_search",
    "search_query": "documente indexate",
    "results": [
      {"id": "doc-1", "relative_path": "docs/a.txt", "score": 0.92}
    ]
  },
  "id": 1
}
```

### `sql_query`

```json
{
  "jsonrpc": "2.0",
  "result": {
    "type": "sql_query",
    "natural_language_answer": "Ai indexat 42 de documente.",
    "row_count": 1
  },
  "id": 1
}
```

### Eroare modul lipsă

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32001,
    "message": "Required module is missing: slm_nl_router",
    "data": {
      "module_id": "slm_nl_router",
      "dependencies": ["onnx_runtime", "duckdb"],
      "download_url_hint": "https://cdn.mirage.ai/modules/slm-nl-router/1.0.0/...",
      "size": 150000000
    }
  },
  "id": 1
}
```

### Flux intern

1. `ask(question)` trimite întrebarea către daemon.
2. Daemonul verifică dacă modulul `slm_nl_router` este ready. Dacă nu, returnează eroare structurată.
3. **Task A** — modelul clasifică intenția (`semantic_search` vs `sql_query`).
4. Dacă `semantic_search`:
   - modelul (sau o euristică) extrage un `search_query` curat în limba sursă;
   - daemonul apelează `embed(search_query)` și `search(query_vector, top_k)`;
   - returnează `type: semantic_search` + lista de rezultate.
5. Dacă `sql_query`:
   - daemonul obține schema DuckDB relevantă;
   - **Task B** — modelul generează SQL;
   - se validează și securizează SQL-ul;
   - daemonul execută SQL-ul prin DuckDB;
   - **Task C** — modelul sumarizează rezultatul în limbaj natural;
   - returnează `type: sql_query` + `natural_language_answer`.
6. Dacă intenția este ambiguă, default-ul este `semantic_search`.

## 7. Post-procesare și securitate

### 7.1 Curățare

- Elimină backticks triple și tagul `sql`.
- Elimină comentariile inline (`-- ...`) pentru a preveni bypass prin comentarii.
- Normalizează whitespace.
- Dacă outputul este gol după curățare, se consideră eroare de generare.

### 7.2 Validare sintactică

Prima literală a interogării trebuie să înceapă cu unul dintre cuvintele cheie permise:

```regex
^(SELECT|CREATE|INSERT|UPDATE|DELETE|EXPLAIN|WITH)\b
```

Dacă nu se potrivește, se returnează eroare `sql_parse_failed`.

### 7.3 Garda de siguranță (destructive DDL/DML)

Default: **numai `SELECT` și `EXPLAIN` sunt permise**. Orice alt verb (`CREATE`, `INSERT`, `UPDATE`, `DELETE`, `DROP`, `ALTER`, `TRUNCATE`, `MERGE`) este blocat.

Regex pentru cuvinte interzise (case-insensitive, word boundary):

```regex
\b(DROP|DELETE|TRUNCATE|ALTER|UPDATE|INSERT|MERGE)\b
```

Dacă utilizatorul setează în `daemon.yaml`:

```yaml
sql_generator:
  allow_destructive_sql: true
```

atunci sunt permise și `CREATE/INSERT/UPDATE/DELETE`, dar `DROP/ALTER/TRUNCATE` rămân blocate fără un flag separat `allow_ddl_drop`.

### 7.4 Fallback la query sigur

Dacă modelul returnează un output invalid sau nesigur, daemonul nu execută interogarea. Opțional poate returna:

```sql
SELECT 'SLM output rejected' AS message WHERE FALSE;
```

aceasta fiind un `SELECT` valid care nu returnează rânduri. Utilizatorul primește un warning în log și, prin IPC, un `data.reason` (ex. `unsafe_keyword_detected`).

### 7.5 Execuție sandbox

SQL generat nu rulează automat la apelul `generate_sql`. Clientul trebuie să trimită explicit `query(sql)` pentru execuție, oferind utilizatorului șansa de a revizui.

## 7. Obținerea și cache-ul schemei

### 7.1 Descoperire schema

Daemonul interoghează DuckDB:

```sql
SHOW TABLES;
DESCRIBE <table_name>;
-- sau
PRAGMA table_info('<table_name>');
```

Pentru fiecare tabel se generează o linie `CREATE TABLE ...` simplificată cu numele coloanelor și tipurile.

### 7.2 Cache

- Cache în memorie în daemon: `HashMap<String, SchemaCacheEntry>`.
- TTL default: 60 secunde.
- Invalidare: la primul `generate_sql` după expirare sau când `context.tables` include un tabel absent din cache.

### 7.3 Reducerea contextului

Dacă schema complet depășește bugetul de tokeni:

1. Se păstrează tabelele menționate explicit în `context.tables`.
2. Se sortează restul după relevanță euristică (ex. nume de tabel care apar în întrebare).
3. Se trunchază până la limită.

## 8. Erori și handling

### 8.1 Erori JSON-RPC structurate

| Situație | `error_kind` | `module_id` (dacă e cazul) | Detalii |
|---|---|---|---|
| Modelul `slm_sql_model` nu este descărcat | `module_missing` | `slm_sql_model` | Include dimensiune și URL din catalog |
| ONNX Runtime nu este descărcat | `module_missing` | `onnx_runtime` | Depinde de modulul runtime |
| DuckDB nu este descărcat | `module_missing` | `duckdb` | Necesar pentru schema și execuție |
| Încărcare ONNX eșuată | `module_load_failed` | `onnx_runtime` / `slm_sql_model` | Detalii în `data.source` |
| Model output gol sau marker lipsă | `sql_parse_failed` | — | `data.reason = "missing_sql_marker"` |
| Output invalid (nu începe cu verb permis) | `sql_parse_failed` | — | `data.reason = "invalid_top_level_keyword"` |
| Interogare nesigură detectată | `unsafe_sql_rejected` | — | `data.reason = "destructive_ddl_dml"` |
| Eroare internă | `internal` | — | Stack trace doar în log |

### 8.2 Comportament

- Lipsa unui modul nu declanșează descărcarea automată; daemonul returnează eroare structurată, iar clientul (GUI/CLI) poate apela `download_module`.
- Toate erorile includ un mesaj lizibil pentru utilizator și un `code` JSON-RPC standard sau de aplicație (`-32001` pentru module missing, `-32002` pentru parse, `-32003` pentru unsafe).

## 9. Metodă IPC

### 9.1 `generate_sql`

**Request:**

```json
{
  "jsonrpc": "2.0",
  "method": "generate_sql",
  "params": {
    "question": "câte documente am indexat?",
    "context": {
      "tables": ["documents"]
    }
  },
  "id": 1
}
```

`context` este opțional. Dacă `context.tables` este omis, se folosește toată schema disponibilă (limitată de token budget).

**Response success:**

```json
{
  "jsonrpc": "2.0",
  "result": {
    "sql": "SELECT COUNT(*) FROM documents;",
    "confidence": 0.87
  },
  "id": 1
}
```

### 9.2 `confidence`

- `confidence` este opțional și reprezintă probabilitatea medie normalizată a tokenilor generați (softmax over logits).
- Dacă modelul nu expune log-probabilități, se poate omite sau seta la `null`.
- Scop: clientul poate afișa un avertisment când `confidence < 0.5`.

## 10. Separarea `query(sql)` și `generate_sql(question)`

- **`generate_sql`**: primește limbaj natural și returnează SQL. El este responsabil de convertirea întrebărilor.
- **`query`**: primește SQL brut și îl execută prin DuckDB. El nu detectează automat limbaj natural.

**Nu se face routing implicit** din `query` către `generate_sql`, pentru a evita execuția accidentală a unui text liber interpretat ca SQL. Dacă se dorește un UX unitar în viitor, CLI poate adăuga un flag explicit:

```bash
mirage query --from-nl "câte documente am indexat?"
# care intern apelează generate_sql, apoi query
```

Acest flag nu modifică semantica metodei IPC `query`.

## 11. Integrare CLI și MCP

### 11.1 CLI

Conform `.agents/specs/cli-mcp-design.md` §2.8:

```bash
mirage sql "câte documente am indexat?"
```

Mapare RPC: `generate_sql(question)`. Dacă modulul lipsește, CLI primește `missing_module` și afișează:

```text
error: module 'slm_sql_model' is missing; run 'mirage download slm_sql_model'
```

Output JSON (`--json`):

```json
{"question": "...", "sql": "...", "confidence": 0.87}
```

### 11.2 MCP

Serverul MCP expune tool-ul `generate_sql` cu schema:

```json
{
  "name": "generate_sql",
  "description": "Generate a DuckDB SQL query from a natural language question.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "question": {"type": "string"},
      "context": {
        "type": "object",
        "properties": {
          "tables": {"type": "array", "items": {"type": "string"}}
        }
      }
    },
    "required": ["question"]
  }
}
```

Mapping: `generate_sql` MCP → IPC `generate_sql`. Răspunsul MCP conține `sql` și opțional `confidence`.

## 12. Strategia de descărcare și quantizare

### 12.1 Dimensiuni

| Model | Dimensiune pachet (model + tokenizer, tar.gz) | Dimensiune extras |
|---|---|---|
| flan-t5-small INT8 | ~70–100 MB | ~100–140 MB |
| flan-t5-base INT8 | ~220–260 MB | ~300–350 MB |
| sqlcoder-2b INT8 | ~450–500 MB | ~600–700 MB |

### 12.2 Quantizare

- Se folosește **dynamic INT8 quantization** cu ONNX Runtime (`onnxruntime.quantization` Python API) pentru weights și activations unde este sigur.
- Se aplică **GraphOptimizationLevel::Level3** la încărcare pentru fuziune de layere și constant folding.
- Optional: se poate exporta modelul cu `use_cache=True` pentru decoder-only, reducând re-computația în buclă.

### 12.3 CDN

- Arhiva `slm-sql-model-1.0.0-universal.tar.gz` este platform-independentă (ONNX model + tokenizer).
- Runtime-ul nativ rămâne în modulul `onnx_runtime`, care este platform-specific.

## 13. Referințe

- ADR 012: `.agents/decisions/adr/012-modular-download-manager.md`
- Modular Download Manager spec: `.agents/specs/modular-download-manager.md`
- CLI & MCP design: `.agents/specs/cli-mcp-design.md`
- Technical spec: `.agents/specs/technical-spec.md` §3.1.4
- ONNX inference pattern (embeddings): `src/daemon_next/src/embeddings.rs`
