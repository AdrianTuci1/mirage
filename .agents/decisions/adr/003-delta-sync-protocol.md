# ADR 003: Protocol de sincronizare delta

## Status

Pending

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

## Decizie propusă

**Opțiunea B** pentru MVP. Endpoint simplu `GET /sync/delta?version={n}` care returnează un stream de fișiere `.lance` noi.

## Consecințe

- Ktor client pe partea de KMP.
- Serverul trebuie să țină evidența versiunilor de index generate.
- Formatul răspunsului: `multipart/stream` sau `tar` stream.

## Note

gRPC poate fi adăugat ca alternativă pentru Enterprise dacă se cere performanță maximă în LAN.
