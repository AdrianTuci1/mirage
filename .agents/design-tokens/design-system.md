# Design System & Tokens — Mirage

## Filozofie UI

- **Local-first, calm, trustable**: interfața trebuie să comunice control și intimitate.
- **Dark-first**: aplicația va folosi un dark mode ca default, cu light mode opțional.
- **Dense information**: multe rezultate de căutare, spațiere eficientă.

## Paleta de culori

Rampa neutră din board-urile Penpot (`L.HEX.dark` din `scripts/penpot/spotlight-build.js`),
în oglindă cu `MirageTokens.kt`. Paliera Nord și accentul albastru/violet au fost scoase în
review-ul din 2026-08: nu mai există culoare de brand, starea se arată prin neutru + culori
de semantică a progresului.

### Dark Mode (default)

| Token | Valoare | Utilizare |
|-------|---------|-----------|
| `--bg-primary` | `#18181A` | Fundal principal |
| `--text-primary` | `#FFFFFF` | Text principal |
| `--text-secondary` | `#98989D` | Text secundar, placeholder, metadata |
| `--border` | `#2E2E32` | Separatoare, chenar exterior |
| `--input-border` | `#48484D` | Chenar input |
| `--selected-bg` | `#38383D` | Rând / tab / chip selectat |
| `--selected-bg-strong` | `#4A4A50` | Stare apăsată |
| `--hover-bg` | `#202024` | Hover subtil |
| `--key-bg` | `#26262A` | Fond pentru key hints |
| `--key-text` | `#C8C8CD` | Text key hints |
| `--progress-idle` | `#6E6E73` | Pistă de progres |
| `--progress-active` | `#EAB308` | Indexare / descărcare în curs |
| `--progress-done` | `#22C55E` | Gata |
| `--traffic-light` | `#FF5F57` `#FEBC2E` `#28C840` | Title bar desenat |

### Light Mode

Nespecificat de board-urile curente. Regula de implementare: aceeași rampă, lumină inversată,
fără accent cromatic; rolurile de token rămân identice, deci componentele nu se schimbă.

## Tipografie

| Rol | Font | Dimensiune | Greutate |
|-----|------|------------|----------|
| H1 | Inter / SF Pro | 24sp | 700 |
| H2 | Inter / SF Pro | 20sp | 600 |
| Body | Inter / SF Pro | 14sp | 400 |
| Caption | Inter / SF Pro | 12sp | 400 |
| Mono | JetBrains Mono | 13sp | 400 |

## Dimensiuni & spațiere

- `--radius-sm`: 6dp
- `--radius-md`: 10dp
- `--radius-lg`: 16dp
- `--spacing-xs`: 4dp
- `--spacing-sm`: 8dp
- `--spacing-md`: 16dp
- `--spacing-lg`: 24dp

## Dimensiuni ferestre

- **Spotlight window**: 720dp × 480dp, centrat pe ecranul activ, 1/3 din înălțime de sus.
- **Settings window**: 960dp × 720dp, fereastră normală, cu title bar desenat și tab strip centrat.
- **Corner radius**: 16dp pentru fereastra flotantă, 12dp pentru input-uri.

## Componente cheie

- **SearchBar**: 48dp, fundal `--bg-primary`, chenar `--input-border`, clear button, buton Settings dreapta.
- **ResultRow**: 44dp, titlu + path + sursă, hover `--hover-bg`, selecție `--selected-bg`.
- **VaultBadge**: indică sursa (`local`, `nas`, `dropbox`, `gdrive`).
- **Footer (Spotlight)**: filtre de surse la stânga, key hints la dreapta (`return` open, `shift+return` download, `tab` clipboard, `esc` close).
- **Indexing row (Settings → General)**: bară de progres 4dp cu `--progress-active` și text „62%", sau chip „Start indexing" / „Re-index" când indexul e stale.
- **TabStrip (Settings)**: icon deasupra label, centrat, tab activ pe `--selected-bg`.
- **WorkerRow / NoteBox (Settings → Servers)**: rând de worker + cutie de notă care explică faptul că credential-ele rămân pe device.
- **EmptyState**: mesaj centrat „Start typing to search".

## Iconițe

- Set: Phosphor Icons sau Material Symbols Outlined.
- Dimensiune default: 20dp.
- Culoare: `--text-secondary` idle, `--text-primary` activ (fără accent cromatic).

## Animations

- Durata default: 150ms.
- Easing: `ease-out`.
- Fără animații opționale care blochează input-ul.
