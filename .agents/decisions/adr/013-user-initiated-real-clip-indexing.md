# ADR 013: Indexare multimodală inițiată de utilizator, cu CLIP real

## Status

Accepted

## Context

Până la acest punct spațiul comun text–imagine a fost simulat: ADR 006 a admis
embeddings hash-fake pentru a debloca UI-ul, iar catalogul de module conținea URL-uri
`example.com` cu checksum zero. Review-ul de produs a stabilit trei reguli hard:
indexarea nu pornește niciodată automat, utilizatorul poate procesa local **sau** poate
descărca un worker, iar credential-ele nu ajung la worker. În plus, cerința de produs a
devenit explicită: „vom avea un spațiu comun text–imagine, ar trebui să avem complet".

Măsurătorile pe greutăți reale au arătat că problema nu se oprește la encodere. În CLIP
ViT-B/32, similaritatea text–text este ~0.72–0.86, iar text–imagine ~0.22–0.29. Într-un
index mixt, un `top_k` brut pe cosine elimina **toate** fotografiile din fereașa vizibilă:
la șase interogări de tip „a photo of a …", zero fotografii apăreau în primele șase
rezultate, indiferent cât de bine se potriveau.

## Decizie

**1. Un singur spațiu 512-d, cu greutăți reale.** `ClipEmbedder` (`src/daemon_next/src/embeddings.rs`)
încarcă `clip_text_encoder` + `clip_vision_encoder` + `clip_tokenizer` (CLIP ViT-B/32,
varianta int8 `Xenova/clip-vit-base-patch32`) și produce vectori comparabili pentru text și
imagine. Preprocesarea imaginii este cea canonică: resize pe latura scurtă la 224,
center-crop 224×224, normalizare CHW cu media/std CLIP. Tokenizarea folosește BPE real
(`tokenizers`, vocabular 49408, `RobertaProcessing` cu `<|startoftext|>`/`<|endoftext|>`),
cu padding/taiere la 77 făcute în cod pentru că `tokenizer.json` nu declară niciuna.

**2. Fără embeddings falși, fără catalog nevalidat.** Hash-embedderul din ADR 006 rămâne
doar ca fallback explicit, semnalizat în `status.modules` (`semantic: false`). Catalogul
implicit conține exclusiv module cu URL, dimensiune și sha256 verificate prin
`scripts/fetch-clip-models.sh`; un test refuză intrările cu checksum zero.

**3. Indexarea se pornește doar la cerere.** Watcher-ul și modificările de setări marchează
indexul `stale`; singurul loc care declanșează o tură este RPC-ul `index_files`, invocat din
chip-ul „Start indexing" / „Re-index" din Settings → General. Tura rulează în fundal, iar
progresul se citește prin `index_status` (fază, count, procent).

**4. Fereașa semantică se împarte pe modalități.** `interleave_modalities` din
`src/daemon_next/src/search.rs` supracolează din LanceDB (`top_k × 6`, plafonat la 120) și
apoi alternează pe modalități, ordonând fiecare grupă după scorul ei. Sortarea finală
compară doar tierurile, nu și scorurile din interiorul tierului semantic, pentru că
`sort_by` este stabil și trebuie să păstreze intercalarea.

## Consecințe

- Spațiul text–imagine este demonstrat, nu presupus: `src/daemon_next/tests/clip_space.rs`
  rulează pe șase fotografii reale etichetate (Wikimedia Commons) și cere 6/6 recuperări
  corecte cu marjă, plus echivalența batch vs. single-item.
- Testele cu greutăți reale sunt condiționate de `MIRAGE_CLIP_MODELS` / `MIRAGE_TEST_IMAGES`
  și se saltă singur când artefactele lipsesc, deci suita implicită rămâne rapidă.
- Ranking-ul cross-modal rămâne aproximativ: intercalarea garantează vizibilitate, dar un
  document slab poate apărea deasupra unei fotografii bune. Pasul următor este un prag
  absolut pe modalitate (calibrat pe aceleași fotografii), nu o normalizare relativă.
- Catalogul nu mai poate minți: o intrare fără checksum verificat pică la test.
