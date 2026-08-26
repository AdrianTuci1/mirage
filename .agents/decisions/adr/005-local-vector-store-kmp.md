# ADR 005: Local Vector Store for the KMP Client

## Status

Accepted

## Context

Task T3.4 requires a local vector store inside the Kotlin Multiplatform (KMP) client so that the floating search UI can show real results. The original plan was to embed LanceDB locally and read the synced `.lance` tables directly from the desktop client.

LanceDB has no official Java/JVM client published on Maven Central. Neither `com.github.lancedb` nor `com.lancedb:lancedb` artifacts are available, and there is no stable JNI/JNA binding we can consume from common Kotlin code without adding heavy native dependencies. This blocks the direct LanceDB-local approach for the KMP client.

We still want to keep LanceDB for the Remote Indexer (Python), because it is the source of truth for vector indexing and delta generation. The client-side store therefore needs to be a pragmatic, swappable abstraction that can be replaced with a real LanceDB-JVM client or a JNI wrapper once one becomes available.

## Opțiuni evaluate

### Opțiunea A: Custom JNI wrapper around LanceDB native core

**Pro:**
- Would give us the real LanceDB format and full feature parity with the remote indexer.
- No data-model duplication between remote and client.

**Contra:**
- Requires writing and maintaining C/C++ JNI code for Windows, macOS and Linux.
- Build, packaging and distribution complexity is very high for an MVP.
- Significantly more work than the rest of T3.4 combined.

### Opțiunea B: Qdrant embedded Java client

**Pro:**
- Pure-JVM/embedded vector store with good performance.
- Mature Kotlin-friendly API.

**Contra:**
- It is not file-sync friendly: the remote indexer produces `.lance` delta files, not Qdrant snapshots.
- Would force us to design a parallel delta/sync format for the client.
- Adds a heavy native dependency and another persistence format.

### Opțiunea C: In-memory brute-force vector store as MVP

**Pro:**
- Keeps the client completely platform-agnostic and dependency-free.
- Mirrors the LanceDB schema exactly, so replacing the backend later is straightforward.
- Unblocks the search UI immediately.
- Cosine similarity can be implemented with `kotlin.math` only.

**Contra:**
- Not persisted to disk yet.
- Brute-force search is O(n) and will not scale to hundreds of thousands of records.
- No ANN index.

## Decizie

Use an **in-memory vector store as the MVP** behind a `LocalVectorStore` abstraction. The implementation lives in `src/client-kmp/src/commonMain/kotlin/search/` and exposes the same schema as LanceDB:

- `VectorRecord`: `id`, `relativePath`, `sourceType`, `vector`, `updatedAt`.
- `SearchResult`: `id`, `relativePath`, `sourceType`, `score`, `vector`.
- `LocalVectorStore` interface with `upsert`, `query` and `size`.
- `InMemoryVectorStore`: brute-force cosine-similarity implementation.
- `SearchEngine`: high-level wrapper used by the UI. `search(query)` is `suspend` for future-proofing, even though the MVP implementation is synchronous.

This leaves the door open to:
- A future LanceDB-JVM client swapping in as a `LocalVectorStore` implementation.
- A JNI/native wrapper once the packaging overhead is justified.
- Migration to a persisted disk store (Qdrant embedded, HNSW, etc.) without touching the UI code.

## Consecințe

- The KMP client can now run real search queries and unblock the floating search UI.
- No heavy native dependencies are added to the build.
- The data model matches the remote LanceDB schema, making delta sync straightforward later.
- Performance is acceptable for MVP/demo datasets but must be revisited before large-scale use.
- Persistence and full-text/token-filtering search are deferred to a future iteration.

## Note

This decision does not change the remote indexer stack (ADR 001). The Remote Indexer keeps using native LanceDB. The client will later consume delta files produced by the remote and populate the `LocalVectorStore` abstraction, regardless of whether the backing implementation is in-memory, LanceDB-JVM or JNI.
