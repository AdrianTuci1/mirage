# Design System & Tokens — Mirage

## Filozofie UI

- **Local-first, calm, trustable**: interfața trebuie să comunice control și intimitate.
- **Dark-first**: aplicația va folosi un dark mode ca default, cu light mode opțional.
- **Dense information**: multe rezultate de căutare, spațiere eficientă.

## Paleta de culori

### Dark Mode (default)

| Token | Valoare | Utilizare |
|-------|---------|-----------|
| `--bg-primary` | `#0F1115` | Fundal principal |
| `--bg-secondary` | `#161920` | Panouri, carduri |
| `--bg-tertiary` | `#1E222A` | Hover, input-uri |
| `--text-primary` | `#F0F2F5` | Text principal |
| `--text-secondary` | `#8F96A6` | Text secundar |
| `--accent` | `#5E81AC` | Accente, butoane primare |
| `--accent-hover` | `#81A1C1` | Hover pe accent |
| `--success` | `#A3BE8C` | Stări pozitive |
| `--warning` | `#EBCB8B` | Avertismente |
| `--error` | `#BF616A` | Erori |

### Light Mode

| Token | Valoare |
|-------|---------|
| `--bg-primary` | `#FFFFFF` |
| `--bg-secondary` | `#F5F7FA` |
| `--bg-tertiary` | `#ECEFF4` |
| `--text-primary` | `#2E3440` |
| `--text-secondary` | `#4C566A` |
| `--accent` | `#5E81AC` |

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

## Componente cheie

- **SearchBar**: fundal `--bg-tertiary`, iconiță de căutare, clear button.
- **ResultCard**: thumbnail stânga, titlu + path + sursă, hover accent.
- **VaultBadge**: indică sursa (`local`, `nas`, `dropbox`, `gdrive`).
- **LicenseDialog**: trial counter + input license key + validate button.

## Iconițe

- Set: Phosphor Icons sau Material Symbols Outlined.
- Dimensiune default: 20dp.
- Culoare: `--text-secondary` idle, `--accent` active.

## Animations

- Durata default: 150ms.
- Easing: `ease-out`.
- Fără animații opționale care blochează input-ul.
