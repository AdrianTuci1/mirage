# ADR 001: Stack pentru Remote Indexer

## Status

Pending

## Context

Remote indexerul trebuie să:
- Scaneze fișiere din surse multiple.
- Extragă feature-uri și genereze embedding-uri.
- Stocheze vectori în LanceDB.
- Expose endpoint-uri HTTP2/gRPC.

Trebuie ales între Python 3.11 și Rust.

## Opțiuni evaluate

### Opțiunea A: Python 3.11

**Pro:**
- Ecosistem bogat: `lancedb`, `onnxruntime`, `fsspec`, `transformers`.
- Rapid de prototipat.
- Multe librării pentru cloud storage.

**Contra:**
- GIL poate limita throughput-ul CPU-bound.
- Dimensiune imagine Docker mai mare.
- Mai lent decât Rust pentru operațiuni masive de I/O.

### Opțiunea B: Rust

**Pro:**
- Performanță maximă și memory safety.
- Binari mici, imagini Docker reduse.
- Concurrency nativ fără GIL.

**Contra:**
- LanceDB are suport Rust, dar ecosistemul ML este mai puțin matur decât în Python.
- Timp de dezvoltare mai lung.
- Mai puține librării ready-made pentru fiecare sursă cloud.

## Decizie propusă

**Python 3.11** pentru MVP, cu posibilitatea de a reevalua Rust pentru o variantă Enterprise de înaltă performanță.

## Consecințe

- Iterații rapide în faza inițială.
- Docker image va fi bazat pe `python:3.11-slim`.
- Se vor alege librării cu suport bun pentru async și memory efficiency.

## Note

Se va reveni asupra acestei decizii după M2 dacă performanța nu este satisfăcătoare.
