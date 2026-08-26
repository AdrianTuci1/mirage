# Comparare Vector Stores — Mirage

## Critere

- Suport nativ pentru fișiere delta.
- Performanță embedded / local.
- Integrare cu Kotlin/Python.
- Licență open-source.
- Overhead de sincronizare.

## Candidați

| Criteriu | LanceDB | Chroma | Qdrant | Milvus |
|----------|---------|--------|--------|--------|
| Fișiere delta native | **Excelent** | Limitat | Nu | Nu |
| Embedded mode | **Da** | Da | Parțial | Nu |
| Python SDK | **Da** | Da | Da | Da |
| Kotlin SDK / JNI | Posibil via C++ | Dificil | gRPC | gRPC |
| Dimensiune imagine | **Mică** | Medie | Medie | Mare |
| Licență | Apache 2.0 | Apache 2.0 | Apache 2.0 | Apache 2.0 |

## Recomandare

**LanceDB** — potrivit pentru arhitectura Mirage datorită formatului de fișiere `.lance` care permite sincronizarea delta directă.
