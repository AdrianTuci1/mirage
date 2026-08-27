# ADR 012: Modular Download Manager

## Status

Accepted

## Context

Binarul daemonului a ajuns la ~280 MB în release (716 MB în debug) după includerea LanceDB, DuckDB și ONNX Runtime. Acest lucru încalcă principiul de a avea un installer mic și de a descărca doar ce folosește utilizatorul. Utilizatorul a cerut explicit ca aplicația să se descarce goală, iar modulele opționale (DuckDB, ONNX Runtime, modele embeddings, SLM etc.) să fie adăugate ulterior, la cerere, în interiorul bundle-ului aplicației.

## Decizie

Implementăm un **Modular Download Manager** în daemon, controlat de GUI.

- Binarul de bază conține doar nucleul: LanceDB, IPC, config, logging și managerul de descărcări.
- Modulele opționale sunt pachete semnate descărcate la cerere în `<app-bundle>/downloads/` și `<app-bundle>/models/`.
- Modulele disponibile inițial:
  - `duckdb` — librăria nativă DuckDB pentru analytics SQL.
  - `onnx_runtime` — runtime ONNX pentru embeddings/vision/SLM.
  - `text_embedding_model` — model ONNX pentru text embeddings.
  - `slm_nl_router` — model ONNX multilingv pentru decizie intenție, generare SQL și sumarizare rezultate în limbaj natural (fără llama.cpp).
  - `vision_model` — model ONNX opțional pentru vision embeddings.
  - `audio_model` — model ONNX opțional pentru audio (viitor).
- Fiecare modul are un manifest cu dimensiune, URL, checksum, platformă și dependințe.
- Managerul verifică checksum, depozitează fișierele și notifică daemonul că poate încărca modulul.
- Dacă un modul lipsește, daemonul returnează o eroare structurată care îndeamnă GUI-ul să solicite descărcarea.

## Consecințe

- Installerul inițial devine mult mai mic.
- Utilizatorul controlează ce descarcă și când.
- Toate descărcările rămân în interiorul aplicației; la ștergerea aplicației se șterg și ele.
- Crește complexitatea: avem nevoie de manifeste semnate, download manager, încărcare dinamică de librării native și gestiunea stărilor "downloading / ready / missing".
- Testele pot rula cu stub-uri sau module locale, fără a descărca la fiecare build.

## Note

- SLM-ul rulează direct prin ONNX Runtime; nu folosim llama.cpp.
- Încărcarea dinamică a DuckDB se poate face prin feature gate Rust sau prin librărie dinamică încărcată la runtime, în funcție de platformă.
