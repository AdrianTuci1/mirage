# ADR 011: Modular Setup Wizard & Resource Control

## Status

Accepted

## Context

Aplicația desktop este local-first și trebuie să respecte resursele utilizatorului (RAM, SSD, bandă). Nu toți utilizatorii au nevoie de toate modulele (audio, vision, translator, SLM).

## Decizie

La prima configurare se afișează un **Modular Setup Wizard** care permite utilizatorului să selecteze ce module activează.

Module disponibile:

- Vector & Text Indexing (LanceDB / Tantivy) — implicit activat.
- Tabular & SQL Analytics Engine (DuckDB) — implicit activat (pre-impachetat în binar).
- Audio / Voice Processing Engine — opțional.
- Multi-Modal & Vision Embeddings — opțional.
- SLM pentru SQL natural-language — opțional.

Reguli:

- Modelele ONNX descărcabile se salvează în folderul aplicației (langă binar), nu în `~/.mirage` sau `Documents`.
- Datele indexate locale și modelele se păstrează în interiorul bundle-ului aplicației.
- La dezinstalare, întreg conținutul aplicației (binare, modele, date) este șters împreună cu aplicația.
- Fiecare modul descarcă doar binarele/modelul necesar, cu confirmare explicită.
- Sync-uri mari și descărcări noi necesită confirmare sau pot fi programate.

## Consecințe

- Crește complexitatea wizardului.
- Reduce riscul de a lăsa date în urmă la dezinstalare.
- Dimensiunea inițială a aplicației crește (DuckDB bunduit).
- Modelele descărcate cresc dimensiunea folderului aplicației.
- Utilizatorul are control deplin asupra resurselor.
- Modulele opționale pot fi activate ulterior din Settings.

## Note

- DuckDB este singurul modul pre-impachetat; restul modelelor se descarcă ulterior în folderul aplicației.
- Pe fiecare platformă se determină programatic calea către directorul aplicației (lângă executabil) pentru a stoca datele locale.
