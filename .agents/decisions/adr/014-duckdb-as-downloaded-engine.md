# ADR 014: DuckDB rulează ca binar descărcat, nu legat în daemon

## Status

Accepted (amendază ADR 009 și §9 din `specs/optional-engine-refactor.md`)

## Context

ADR 009 a stabilit că DuckDB se descarcă la cerere și că binarul de bază rămâne mic. Implementarea a încălcat acea decizie: `Cargo.toml` avea
`duckdb = { version = "1", features = ["bundled"] }` în `default`, deci daemonul compila și lega static DuckDB C++ în sine. Măsurat pe
`target/debug/mirage-daemon`: arhiva `libduckdb.a` 236 MB (247.374.288 B), `__text` 203 MiB dintr-un binar de 689 MiB, 733 de pachete în
`Cargo.lock`. Entries `duckdb` și `onnx_runtime` în catalog fuseseră șterse pentru că URL-urile lor erau placeholder-e care nu funcționau,
iar `ModuleManager` marca modulul `duckdb` ca `Ready` doar prin `#[cfg(feature = "duckdb")]` — adică „pregătit” însemna „legat la compile-time”.

`specs/optional-engine-refactor.md` §9 propunea `libloading` + `libduckdb.dylib` descărcat. Varianta nula funcționa: crate-ul `duckdb`
rezolvă librăria la **link-time** (rustc scrie `LC_LOAD_DYLIB` în executabil), deci `DUCKDB_LIB_DIR`/`DYLD_LIBRARY_PATH` setate la runtime nu
schimbă nimic, pe macOS calea `@rpath` cere `install_name_tool` la pachetare (specul admite aceeași limitare în §9.4), iar `env::set_var`
într-un proces multithreaded nu e sigur.

## Decizie

DuckDB devine un **motor extern, descărcat ca binar standalone**, pe care daemonul îl rulează ca proces copil.

1. `duckdb` iese complet din `Cargo.toml`: fără dependință, fără feature. `default = ["onnx"]`.
2. Catalogul încorporat declară modulul `duckdb` ca `kind: runtime`, `is_optional: true`, versiune `1.5.5`, cu **5 platforme** care indică arhivele
   oficiale `https://github.com/duckdb/duckdb/releases/download/v1.5.5/duckdb_cli-<platform>.zip`, fiecare legată de două SHA-256: arhiva
   (`platform.checksum`, 12,9–21,3 MB) și binarul extras (`files[].sha256`, 37–62 MB), cu `executable: true` pentru ca
   `verify_extracted_files` să pună bit-ul de execuție.
3. `src/analytics.rs` păstrează același API (`open`, `query`, `execute`, `ingest_csv`, `ingest_parquet`, `db_path`, `is_available`), dar
   fiecare apel pornește engine-ul: `<binar> -json -batch -bail -no-init <data_dir>/analytics.duckdb -c <sql>`, cu `stdin` null. Ieșirea JSON
   devine direct `Vec<serde_json::Map>`; nu mai există mapa manuală pe tipuri arrow.
4. Call-urile sunt **serializate printr-un `Mutex`** în `Analytics`. Măsurat: 6 invocații concurente pe același fișier → 5 eșuează cu
   „Could not set lock on file”, deci without lock ar fi fost o cursă reală.
5. Calea engine-ului se rezolvă **la fiecare apel** (`MIRAGE_DUCKDB_BIN` → altfel cel mai nou `<downloads_dir>/duckdb/<version>/duckdb[.exe]`),
   deci un download terminat după pornire activează tabular fără restart.
6. `ModuleManager::reconcile_duckdb_engine` raportează starea din **prezența fișierului de pe disc**, nu din `cfg!`: Ready dacă engine-ul
   există, Missing dacă un `state.json` vechi mințea. Stările care descriu un download în curs (Queued/Downloading/Verifying/Paused/Error)
   nu sunt atinse. Rulează la pornire și la `refresh_catalog`.
7. `status.modules.tabular` rămâne `modules.tabular && analytics.is_available()` — devine în sfârșit o informație adevărată.

## Consecințe

- Fiecare interogare SQL costă un launch de proces: măsurat 12 ms/invocație (`duckdb -json -c "SELECT 1"`, ×5), neglijabil pentru `query`/`ask`.
- Contractul CLI verificat cu binarul real v1.5.5: DDL/DML nu tipărește nimic, un SELECT gol tipărește `[]`, eroarea merge pe stderr cu exit 1
  (deci `[]` nu poate fi confundat cu eșecul), `SHOW TABLES` → `[{"name":"t"}]`, `read_csv_auto`/`read_parquet` și `SELECT count(*)` într-o
  singură invocare → `[{"row_count":2}]`.
- `execute()` nu mai poate returna numărul de rânduri afectate (`changes()` nu există ca scalar funcție în v1.5.5, verificat); semnătura
  devine `Result<()>`. Niciun apelant din producție nu citea valoarea.
- `DECIMAL` vine ca șir în JSON (alegerea exportatorului DuckDB pentru precizie); valorile `DOUBLE`/`INTEGER` rămân numerice.
- Fișierul `analytics.duckdb` e scris de versiunea din catalog (1.5.5). O retrofitare a versiunii e decizie de catalog, nu de build.
- Mersul înainte pentru ONNX Runtime e același lucru (un `ort` build legat dinamic sau un sidecar), dar nu e parte din această decizie.
- Binorul debug scade cu cele 236 MB de `libduckdb.a` plus monomorfizările arrow/datafusion aduse de crate-ul `duckdb`; `Cargo.lock` a
  căzut de la 733 la 705 de pachete la o simplă rezolvare (`cargo metadata`, fără compilare).

## Alternative

- `libloading` + `libduckdb` descărcat (specul vechi): nu funcționează cu legarea la link-time a crate-ului `duckdb`, cere muncă de
  pachetare pe macOS și nu e sigurat threading-ul. Respins.
- Trimiterea SQL către un worker remote: contrazice ADR 013 (procesare locală sau pe worker, fără credential-e pe worker) pentru un features
  care oricum rulează local. Respins.
- Păstrarea `duckdb` ca feature opțional în `default`: am fi păstrat problema de mărime în build-ul de producție. Respins.
