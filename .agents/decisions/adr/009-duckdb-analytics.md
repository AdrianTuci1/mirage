# ADR 009: DuckDB Analytics Engine

## Status

Accepted

## Context

Utilizatorii vor să interogheze date tabulare (CSV, Parquet, JSON, SQLite) cu viteză OLAP. Modelele de limbaj nu pot executa calcule exacte pe volume mari, iar un motor SQL embedded este necesar.

## Decizie

Folosim **DuckDB** ca motor OLAP embedded în daemon.

- DuckDB interoghează direct fișiere tabulare brute.
- SLM-ul local transformă întrebările în limbaj natural în SQL.
- Rezultatele exacte sunt returnate în sub 15ms pentru volume medii.

## Consecințe

- Adăugăm dependință DuckDB în daemon Rust.
- Adăugăm metodă IPC `query(sql)`.
- CLI primește comanda `mirage query`.
- MCP expune tool `query`.
- Volumele mari pot fi procesate local dacă încap pe disc sau remote prin worker.

## Note

DuckDB rămâne opțional în Setup Wizard, dar este activat implicit.
