# ADR 003: Protocol de sincronizare delta

## Status

Accepted

## Context

Clienții trebuie să sincronizeze indexul local cu Remote Indexerul. Sincronizarea trebuie să fie eficientă și să transfere doar datele noi.

## Opțiuni evaluate

### Opțiunea A: gRPC cu streaming

**Pro:**
- Contract strâns, generate de cod.
- Streaming binar eficient.
- Bun pentru LAN.

**Contra:**
- Necesită proxy (envoy/ngrok) pentru traversarea NAT/HTTP firewalls.
- Mai complex în KMP.

### Opțiunea B: HTTP/2 cu streaming de fișiere .lance

**Pro:**
- Simplu de implementat în KMP cu Ktor.
- Funcționează prin proxy-uri standard.
- LanceDB poate genera direct fișiere .lance ca artifacte delta.

**Contra:**
- Mai puțin „contract-driven" decât gRPC.
- Generarea fișierelor `.lance` delta增量 în format portabil între LanceDB native și clientul KMP adaugă complexitate în MVP.

### Opțiunea C: HTTP/1.1 (sau HTTP/2) cu streaming NDJSON de înregistrări (MVP simplification)

**Pro:**
- Implementare rapidă pe server (FastAPI + StreamingResponse).
- Client KMP poate consuma linie cu linie fără librării suplimentare.
- Nu depinde de compatibilitatea binară a fișierelor `.lance`.
- Autentificare simplă via `Authorization: Bearer {passkey}`.

**Contra:**
- Payload mai mare decât fișierele `.lance` brute.
- Clientul trebuie să reconstruiască vectorii local.

## Decizie

**Opțiunea C pentru MVP.** Endpoint `GET /sync/delta?version={client_last_version}` returnează un stream `application/x-ndjson` de înregistrări LanceDB complete. Fiecare linie este un obiect JSON cu câmpurile `id`, `relative_path`, `source_type`, `vector`, `updated_at` și `version`. Răspunsul include headerul `X-Latest-Version` cu versiunea curentă a serverului.

Aceasta este o simplificare MVP față de **Opțiunea B** propusă inițial. Transferul de fișiere `.lance` brute poate fi reevaluat după MVP dacă payload-ul NDJSON devine o problemă de performanță.

## Consecințe

- Serverul persistă un număr de versiune global (într-un fișier JSON lângă LanceDB) și îl expune prin `X-Latest-Version`.
- Fiecare rulare a pipeline-ului primește un nou număr de versiune; toate înregistrările adăugate/actualizate în acea rulare primesc același `version`.
- Clientul KMP citește NDJSON linie cu linie și aplică înregistrările în store-ul vectorial local.
- Endpoint-ul este protejat cu `Authorization: Bearer {passkey}`, comparat cu `settings.secret_key`.
- Formatul răspunsului pentru MVP: `application/x-ndjson`.

## Note

gRPC (Opțiunea A) sau streaming de fișiere `.lance` (Opțiunea B) pot fi adăugate ca alternativă pentru Enterprise sau pentru rețele locale rapide după MVP.
