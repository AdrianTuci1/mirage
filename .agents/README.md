# `.agents` — Agent Workbench for Mirage

Acest folder este **contextul viu** al proiectului. Orice agent AI care lucrează pe Mirage ar trebui să înceapă de aici.

## Scop

- Păstrează specificațiile, arhitectura și deciziile la zi.
- Oferă un graf de execuție clar cu task-uri, dependențe și milestones.
- Stochează design tokens, runbooks și șabloane pentru consistență.
- Permite agenților să actualizeze progresul fără a altera codul sursă.

## Structură

```
.agents/
├── README.md                    # Acest fișier
├── specs/                       # Specificații tehnice și de business
│   ├── technical-spec.md        # Spec principal de implementare
│   └── pricing-monetization.md  # Strategia de monetizare
├── execution-graph/             # Graf de execuție și plan de implementare
│   ├── project-graph.json       # Graf mașină-citibil (DAG)
│   └── milestones.md            # Milestones și livrabile
├── architecture/                # Arhitectură, diagrame, contracte
│   └── system-overview.md
├── decisions/                   # Decizii arhitecturale și documentație de evaluare
│   ├── adr/                     # Architecture Decision Records
│   ├── comparison/              # Tabele comparative (stack, librării)
│   └── rfcs/                    # Request for Comments propuse
├── design-tokens/               # Design system și tokens UI
│   └── design-system.md
├── progress/                    # Starea curentă a proiectului
│   └── progress.md
├── runbooks/                    # Ghiduri pentru agenți
│   └── agent-runbook.md
└── templates/                   # Șabloane pentru task-uri noi
    └── task-template.md
```

## Reguli de utilizare pentru agenți

1. Citește `progress/progress.md` înainte de a începe un task nou.
2. Actualizează `progress/progress.md` când finalizezi un task.
3. Dacă o decizie arhitecturală se schimbă, adaugă un ADR nou; nu șterge vechiul.
4. Folosește șabloanele din `templates/` pentru task-uri noi.
5. Păstrează specificațiile din `specs/` sincronizate cu codul.

## Stare curentă

- Proiect inițializat: `2026-08-26`
- Faza curentă: Planificare și setup
- Niciun modul de implementare nu a început încă.
