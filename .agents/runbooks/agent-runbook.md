# Agent Runbook — Mirage

## La începutul fiecărui task

1. Citește `.agents/README.md`.
2. Citește `.agents/progress/progress.md` pentru a înțelege starea curentă.
3. Citește specificațiile relevante din `.agents/specs/`.
4. Verifică ADR-urile relevante din `.agents/decisions/adr/`.

## Când lucrezi la un task

1. Găsește task-ul în `.agents/execution-graph/project-graph.json`.
2. Actualizează statusul task-ului în `in_progress`.
3. Respectă structura de foldere existentă.
4. Adaugă teste acolo unde este aplicabil.
5. Nu șterge documentație veche; adaugă note de actualizare.

## La finalul fiecărui task

1. Rulează build-ul / testele locale.
2. Actualizează statusul task-ului în `completed` în `project-graph.json`.
3. Actualizează `.agents/progress/progress.md`:
   - Adaugă task-ul la "Task-uri finalizate".
   - Mută task-urile dependente în "Task-uri următoare" dacă devin disponibile.
   - Notează blockerele dacă apar.
4. Dacă decizia unui ADR s-a schimbat, adaugă un nou ADR în `.agents/decisions/adr/`, nu modifica vechiul.
5. Commit cu un mesaj clar: `[M1] Implement LanceDB schema and delta logic`.

## Convenții de cod

- **Python**: PEP 8, type hints, `ruff` pentru lint.
- **Kotlin**: Kotlin Coding Conventions, Compose UI în `commonMain` cât mai mult posibil.
- **Docker**: imagini mici, volum read-only pentru surse.
- **Git**: commit-uri atomice, branch-uri cu prefix `feature/`, `fix/`, `docs/`.

## Verificare înainte de PR

- [ ] Codul compilează.
- [ ] Testele trec.
- [ ] Documentația din `.agents/specs/` reflectă implementarea.
- [ ] ADR-urile sunt la zi.
- [ ] Progresul este actualizat.
