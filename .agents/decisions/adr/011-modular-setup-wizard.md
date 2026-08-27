# ADR 011: Modular Setup Wizard & Resource Control

## Status

Accepted (amendat — toate modulele mari sunt descărcabile)

## Context

Aplicația desktop este local-first și trebuie să respecte resursele utilizatorului (RAM, SSD, bandă). Binarul complet (LanceDB + DuckDB + ONNX Runtime) depășește 200 MB, ceea ce este prea mare pentru un installer. Utilizatorul trebuie să descarce doar ce folosește.

## Decizie

La prima configurare se afișează un **Modular Setup Wizard** care permite utilizatorului să selecteze ce module activează. Niciun modul mare nu este pre-impachetat în binarul de bază.

Module disponibile:

- Vector & Text Indexing (LanceDB / Tantivy) — implicit activat; **inclus în binarul de bază** (este nucleul aplicației).
- Tabular & SQL Analytics Engine (DuckDB) — opțional; **descărcabil** la cerere.
- ONNX Runtime — necesar pentru embeddings / vision / SLM; **descărcabil** la cerere.
- Audio / Voice Processing Engine — opțional.
- Multi-Modal & Vision Embeddings — opțional.
- SLM pentru SQL natural-language — opțional; rulează direct prin ONNX Runtime, **fără llama.cpp**.

Reguli:

- Binarul de bază conține doar nucleul (LanceDB + IPC + manager de descărcări).
- Modulele opționale și modelele ONNX se salvează în `<app-bundle>/downloads/` și `<app-bundle>/models/`, nu în `~/.mirage` sau `Documents`.
- Datele indexate locale și modelele se păstrează în interiorul bundle-ului aplicației.
- La dezinstalare, întreg conținutul aplicației (binare, modele, date, descărcări) este șters împreună cu aplicația.
- Fiecare modul descarcă doar binarele/modelul necesar, cu confirmare explicită.
- Sync-uri mari și descărcări noi necesită confirmare sau pot fi programate.

## Consecințe

- Crește complexitatea wizardului și a managerului de descărcări.
- Reduce riscul de a lăsa date în urmă la dezinstalare.
- Dimensiunea inițială a aplicației scade semnificativ (sub 50 MB).
- Modelele descărcate cresc dimensiunea folderului aplicației după utilizare.
- Utilizatorul are control deplin asupra resurselor.
- Modulele opționale pot fi activate ulterior din Settings.

## Note

- DuckDB nu mai este pre-impachetat; devine modul descărcabil (ADR 009 amendat).
- ONNX Runtime este descărcabil și partajat între embeddings, vision și SLM.
- Pe fiecare platformă se determină programatic calea către directorul aplicației (lângă executabil) pentru a stoca datele locale.
- SLM-ul este un model ONNX mic, pornit direct de daemon fără llama.cpp / GGUF.
