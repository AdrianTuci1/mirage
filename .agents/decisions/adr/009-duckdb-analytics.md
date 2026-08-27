# ADR 009: DuckDB Analytics Engine

## Status

Accepted (amendat — modular download)

## Context

Utilizatorii vor să interogheze date tabulare (CSV, Parquet, JSON, SQLite) cu viteză OLAP. Modelele de limbaj nu pot executa calcule exacte pe volume mari, iar un motor SQL embedded este necesar. Pentru a menține binarul inițial mic, modulele mari nu trebuie incluse în installer.

## Decizie

Folosim **DuckDB** ca motor OLAP embedded în daemon, dar **se descarcă la cerere**, nu este bunduit în binarul de bază.

- Binarul de bază conține doar nucleul (LanceDB + IPC + manager de descărcări).
- DuckDB este descărcat în `<app-bundle>/downloads/` când utilizatorul activează modulul din Setup Wizard sau la prima interogare tabulară.
- Daemonul detectează absența DuckDB și returnează o eroare structurată sau pornește wizard-ul de confirmare.
- DuckDB interoghează direct fișiere tabulare brute.
- SLM-ul local (ONNX) transformă întrebările în limbaj natural în SQL.
- Rezultatele exacte sunt returnate în sub 15ms pentru volume medii.

## Consecințe

- Dimensiunea binarului de bază rămâne mică (~20–50 MB în funcție de platformă).
- Utilizatorul trebuie să aprobe descărcarea DuckDB la prima utilizare.
- Setup Wizard gestionează DuckDB ca modul descărcabil.
- La dezinstalare, folderul aplicației (inclusiv descărcările) este șters.

## Note

- DuckDB rămâne opțional: fără el, `query()` și `generate_sql()` returnează eroare.
- Managerul de descărcări centralizează toate modulele opționale (DuckDB, ONNX Runtime, SLM, vision, audio).
