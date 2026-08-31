# ADR 015: ONNX Runtime se încarcă dinamic dintr-un modul descărcat, nu se leagă în daemon

## Status

Accepted (extinde principiul din ADR 009/013 asupra lui ONNX Runtime; rezolvă „mersul înainte" lăsat deschis în consecințele ADR 014)

## Context

`Cargo.toml` lega static ONNX Runtime prin feature-ul `onnx` cu `ort/download-binaries` + `ort/copy-dylibs`. Binarul debug creștea cu
rlib-ul static `onnxruntime` (~78 MB), iar o parte din CLIP greutățile erau gândite ca module descărcate separat — runtime-ul însă rămânea în
binar. Infrastructura de descărcare dinamică exista deja în `src/embeddings.rs` (`module_runtime_dylib` + `ORT_DYLIB_PATH`), dar feature-ul
încă folosea `download-binaries`, deci calea către bibliotecă nu era niciodată folosită.

`ort 2.0.0-rc.13` expune feature-ul `load-dynamic`; în `ort-sys/build/main.rs` acesta activează `disable-linking`, care oprește build-ul
înainte de a descărca sau a lega runtime-ul. Verificat pe arhivele oficiale `microsoft/onnxruntime` v1.28.0:

- macOS arm64: pagina conține `lib/libonnxruntime.dylib` ca fișier real.
- Linux x64/arm64: doar `lib/libonnxruntime.so.1.28.0` este fișier real; `libonnxruntime.so` și `libonnxruntime.so.1` sunt symlink-uri pe care
  `extract_tar_gz` nu le materializează (extrage doar fișiere).
- Windows x64: `lib/onnxruntime.dll` fișier real; arhiva conține și un `onnxruntime.pdb` de ~408 MB (nefolosit, dar extras de modul).
- **macOS x64 nu are asset publicat** la v1.28.0, deci nu poate fi oferit în catalog.

## Decizie

ONNX Runtime devine un **modul descărcabil**, încărcat la runtime prin `libloading` (feature-ul `load-dynamic` al crate-ului `ort`).

1. Feature-ul `onnx` din `Cargo.toml` trece de la `ort/download-binaries` + `ort/copy-dylibs` la **`ort/load-dynamic`** (păstrează
   `std`, `ndarray`, `tracing`, `tls-rustls`, `api-27`). Build-ul nu mai descarcă și nu mai leagă runtime-ul.
2. Catalogul încorporat declară modulul `onnx_runtime` `kind: runtime`, `is_optional: true`, versiune `1.28.0`, cu **4 platforme**
   (`macos_aarch64`, `linux_x86_64`, `linux_aarch64`, `windows_x86_64`), fiecare legată de două SHA-256: arhiva (`platform.checksum`) și
   biblioteca extrasă (`files[].sha256`). `files[].relative_path` punctează către calea din interiorul arhivei (care include numele
   directorului de platformă, ex. `onnxruntime-osx-arm64-1.28.0/lib/libonnxruntime.dylib`). macOS x64 este omis (nu are asset).
3. `src/embeddings.rs`: `create_embedder(models_dir, downloads_dir)` localizează biblioteca în `downloads_dir/onnx_runtime/` și setează
   `ORT_DYLIB_PATH` înainte de prima `Session`. `MIRAGE_DOWNLOADS_DIR` rămâne un override pentru teste/dev. Pe Linux numele este potrivit
   prin prefix `libonnxruntime.so` (fișierul extrage este cel versionat).
4. `src/modules/manager.rs`: auto-ready-ul din `#[cfg(feature = "onnx")]` este eliminat; `reconcile_onnx_runtime` raportează starea din
   prezența fișierului pe disc, exact ca `reconcile_duckdb_engine`. Rulează la pornire și la `refresh_catalog`.
5. Fără modul, `ClipEmbedder::new` eșuează la `dlopen` și `create_embedder` cade pe embedder-ul determinist — indexarea și căutarea nu se
   sting, doar pierd semantica.

## Consecințe

- Binarul de bază pierde rlib-ul static ONNX Runtime (~78 MB). CLIP funcționează doar după ce utilizatorul descarcă modulul `onnx_runtime`.
- macOS x64 nu mai este suportat pentru embeddings ONNX (nu există asset 1.28); build-ul pe Intel Mac oricum eșua la `download-binaries`.
- Linux: fișierul extras este `libonnxruntime.so.1.28.0` (nu `.so`), deci `ORT_DYLIB_PATH` trebuie să indice varianta versionată; `libloading`
  o încarcă fără probleme.
- Windows: modulul extrage întreaga arhivă (~430 MB cu `.pdb`), deși se verifică doar `onnxruntime.dll`; `check_disk_space` (3× dimensiunea
  arhivei) poate subestima nevoia reală pe disc. Acceptabil, documentat.
- **Nesigur la build:** regula „nu mai compila daemon-ul, verifică doar în cod" a rămas activă, deci `cargo build`/`cargo test` nu au rulat;
  `cargo fmt --check` și `cargo metadata` sunt singurele validări. Schimbarea de linking trebuie confirmată de un build real.

## Alternative

- Păstrarea `download-binaries` + `copy-dylibs`: build-ul de producție ar fi păstrat problema de mărime. Respins.
- Rularea inferenței ONNX într-un subproces (sidecar) ca la DuckDB: dezechilibrat — `ort` oferă deja `load-dynamic`, deci nu e nevoie de un
  proces separat. Respins.
- Lăsarea modulului `onnx_runtime` doar în catalog, fără a comuta feature-ul: nu ar fi scos runtime-ul din binar. Respins.
