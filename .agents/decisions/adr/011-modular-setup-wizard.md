# ADR 011: Modular Setup Wizard & Resource Control

## Status

Accepted

## Context

Aplicația desktop este local-first și trebuie să respecte resursele utilizatorului (RAM, SSD, bandă). Nu toți utilizatorii au nevoie de toate modulele (audio, vision, translator, SLM).

## Decizie

La prima configurare se afișează un **Modular Setup Wizard** care permite utilizatorului să selecteze ce module activează.

Module disponibile:

- Vector & Text Indexing (LanceDB / Tantivy) — implicit activat.
- Tabular & SQL Analytics Engine (DuckDB) — implicit activat.
- Audio / Voice Processing Engine — opțional.
- Multi-Modal & Vision Embeddings — opțional.
- SLM pentru SQL natural-language — opțional.

Reguli:

- Modelele ONNX se descarcă doar cu confirmare.
- Fiecare modul descarcă doar binarele necesare.
- Sync-uri mari și descărcări noi necesită confirmare sau pot fi programate.

## Consecințe

- Crește complexitatea wizardului.
- Reduce footprint-ul implicit.
- Utilizatorul are control deplin asupra resurselor.
- Modulele opționale pot fi activate ulterior din Settings.

## Note

Wizardul este parte a GUI, dar daemonul trebuie să suporte activarea dinamică a modulelor.
