# ADR 009: DuckDB Analytics Engine

## Status

Accepted

## Context

Utilizatorii vor să interogheze date tabulare (CSV, Parquet, JSON, SQLite) cu viteză OLAP. Modelele de limbaj nu pot executa calcule exacte pe volume mari, iar un motor SQL embedded este necesar.

## Decizie

Folosim **DuckDB** ca motor OLAP embedded în daemon, **pre-impachetat în binar**.

- DuckDB este legat static sau bunduit ca librărie nativă în daemonul Rust.
- Nu se descarcă la runtime; utilizatorul nu are nevoie de instalare separată.
- DuckDB interoghează direct fișiere tabulare brute.
- SLM-ul local (ONNX) transformă întrebările în limbaj natural în SQL.
- Rezultatele exacte sunt returnate în sub 15ms pentru volume medii.

## Consecințe

- Crește dimensiunea binarului cu ~30-50 MB.
- Nu mai există dependință de descărcare DuckDB la runtime.
- Setup wizard nu mai trebuie să gestioneze DuckDB ca modul descărcabil.
- DuckDB rămâne opțional la nivel de funcționalitate, dar binarul îl conține.

## Note

DuckDB este activat implicit. Utilizatorul poate dezactiva funcționalitatea din setări, dar binarul îl include.
