# Mirage UI Design System

## 1. Filozofie

Interfața Mirage este **curată, minimală și aproape de sistemul de operare**. Nu există carduri greoaie, umbre mari sau iconițe decorative. UI-ul se comportă ca un înlocuitor pentru Spotlight / Alfred: o fereastră mică, rapidă, cu focus pe tastatură.

Principii:
- **Un singur accent de culoare**: mov desaturat / lila pentru selecție.
- **Fără chenare vizibile** în jurul elementelor individuale.
- **Iconițe native** pentru aplicații (furnizate de OS).
- **Prioritizare inteligentă**: aplicațiile apar primele, apoi fișiere, apoi rezultate semantice (poze, clipuri).
- **Rezultatele apar doar când userul scrie** — altfel fereastra arată doar input-ul și footer-ul.

## 2. Design Tokens

### 2.1 Culori

Rampa neutră implicită (dark), identică cu `MirageTokens` din client și cu tokenii
`L.HEX.dark` din fișierul Penpot:

| Token | Valoare | Utilizare |
|-------|---------|-----------|
| `--color-bg` | `#18181A` | Fundal fereastră principală și settings |
| `--color-text-primary` | `#FFFFFF` | Text principal, titluri, input |
| `--color-text-secondary` | `#98989D` | Descrieri, placeholder, metadata |
| `--color-border` | `#2E2E32` | Chenar exterior input, separator, `<tab>` box |
| `--color-input-border` | `#48484D` | Chenar interior input (1px) |
| `--color-selected-bg` | `#38383D` | Fundal element selectat (rând, tab, chip) |
| `--color-selected-bg-strong` | `#4A4A50` | Variantă selectată mai tare (buton apăsatic) |
| `--color-selected-text` | `#FFFFFF` | Text pe element selectat |
| `--color-key-bg` | `#26262A` | Fundal pentru taste afișate (`<tab>`, `<x>`, `<option>`) |
| `--color-key-text` | `#C8C8CD` | Text taste afișate |
| `--color-hover-bg` | `#202024` | Hover subtil pe elemente neselectate |
| `--color-progress-idle` | `#6E6E73` | Pista de progres / stare „în așteptare" |
| `--color-progress-active` | `#EAB308` | Lucru în curs (indexare, descărcare modul) |
| `--color-progress-done` | `#22C55E` | Terminat / gata |

> Notă: accentul mov din versiunile timpurii a fost scos în review-ul din 2026-08 — nu mai
> există culoare cromatică de brand, iar starea se exprimă prin rampă neutrilă + culori de
> semantică a progresului. Light mode inversează doar luminozitatea rămpii, păstrând
> aceleași roluri de token.

### 2.2 Spațiere

| Token | Valoare |
|-------|---------|
| `--space-xs` | `4px` |
| `--space-sm` | `8px` |
| `--space-md` | `12px` |
| `--space-lg` | `16px` |
| `--space-xl` | `24px` |
| `--input-padding` | `10px` |
| `--window-min-width` | `560px` |
| `--window-max-width` | `720px` |
| `--result-height` | `44px` |

### 2.3 Tipografie

| Token | Font | Dimensiune | Greutate | Utilizare |
|-------|------|------------|----------|-----------|
| `--font-family` | Sistem (SF Pro / Segoe UI / Inter) | — | — | Toate textele |
| `--text-input` | sistem | `18px` | `400` | Text în input |
| `--text-result-title` | sistem | `14px` | `500` | Numele unui rezultat |
| `--text-result-meta` | sistem | `12px` | `400` | Descriere / cale / metadata |
| `--text-footer` | sistem | `12px` | `400` | Footer shortcuts |
| `--text-setting-title` | sistem | `14px` | `500` | Titlu opțiune settings |
| `--text-setting-desc` | sistem | `12px` | `400` | Descriere opțiune settings |

### 2.4 Raze de colț

| Token | Valoare | Utilizare |
|-------|---------|-----------|
| `--radius-sm` | `4px` | Taste afișate, switch |
| `--radius-md` | `8px` | Input exterior, rânduri selectate, tab-uri settings |
| `--radius-lg` | `12px` | Fereastră principală |

## 3. Fereastra principală de căutare

### 3.1 Structură

```
┌────────────────────────────────────────────┐
│  ┌──────────────────────────────────────┐  │
│  │  Search everything                   │  │  ← Input principal
│  └──────────────────────────────────────┘  │
│                                              │
│  Indexing photos...  [====>    ]  34%      │  ← Progres indexare (opțional)
│  onnx  slm  duckdb                           │  ← Status module (opțional)
│  ─────────────────────────────────────────  │  ← Separator (afișat doar dacă există rezultate)
│  [icon] Safari                    search   │
│  ···································· item │  ← Rezultat selectat (fundal mov)
│  [icon] Photos                             │
│  [icon] invoice.pdf               search   │
│  ···································· item │
│  [icon] beach.jpg                 search   │
│  ···································· image│
│  ─────────────────────────────────────────  │
│                                 ⌘O + ↵ open │  ← Footer
└────────────────────────────────────────────┘
```

### 3.2 Input-ul principal

- **Chenar exterior**: `1px solid var(--color-border)`, radius `8px`, padding total `10px`.
- **Chenar interior (negru)**: un `TextField` cu border-bottom sau un container interior cu `1px solid black`. Varianta preferată: input fără border vizibil, dar înconjurat de un chenar negru subțire (1px) care face parte din containerul de input.
- **Fără iconiță** în interiorul input-ului.
- **Placeholder**: `"Search everything"`, culoare `--color-text-secondary`.
- **Font input**: `18px`, culoare neagră.
- Input-ul primește focus automat la deschiderea ferestrei.

### 3.3 Zona de progres și module

Afișată **sub input**, deasupra rezultatelor:

- **Progres indexare**: text stânga + bară de progres + procent, font `12px`.
- **Status module**: tag-uri mici (de ex. `onnx`, `slm`, `duckdb`) care indică starea modulelor descărcabile. Tag-urile sunt gri când modulul nu e ready și mov când e ready.

Zona este vizibilă doar când există activitate (indexare în desfășurare sau module lipsă recomandate).

### 3.4 Lista de rezultate

Rezultatele apar **doar când utilizatorul a scris cel puțin un caracter**.

#### Prioritizare

1. **Aplicații** (icon OS nativ, nume aplicație).
2. **Fișiere locale** (icon generic sau thumbnail, nume fișier, cale scurtă).
3. **Rezultate semantice** — poze, clipuri, audio, documente (icon/thumbnail + snippet/metadata).

#### Rând de rezultat

```
┌─────────────────────────────────────────────────────────┐
│ [icon 32px]  Nume rezultat         [search] [item] [tab]│
│              Cale sau metadata                          │
└─────────────────────────────────────────────────────────┘
```

- **Icon**: `32x32px`, furnizat de OS pentru aplicații sau generic pentru fișiere.
- **Titlu**: `14px`, font-weight `500`.
- **Metadata**: `12px`, `--color-text-secondary`, o singură linie.
- **Badges dreapta**:
  - Textul `search item` (sau `open app`, `show image`, etc.) urmat de o cutie gri care conține `<tab>`.
  - Cutia gri: `--color-key-bg`, radius `4px`, padding `2px 6px`.

#### Stări

- **Neselectat**: fundal transparent, fără chenar.
- **Hover**: fundal `--color-hover-bg`.
- **Selectat**: fundal `--color-selected-bg`, text negru, **fără chenar**.
- Primul rezultat este selectat by default la apariția listei.

### 3.5 Footer

- Separator subțire deasupra.
- Aliniere la dreapta: `<option> + <x> open settings`.
- Taste afișate în cutii gri (`--color-key-bg`).
- Exemple:
  - `↵ open` pentru rezultat selectat
  - `⌘O + ↵ open settings`

### 3.6 Comportamente

- **Drag/repoziționare fereastră**: click+hold oriunde în fereastră **afară de input, rezultate și footer** mută fereastra.
- **Navigare**: `↑` / `↓` prin rezultate, `↵` pentru acțiunea principală.
- **Escape**: închide fereastra de căutare.
- **Settings**: shortcut `⌘,` sau footer `⌘O + ↵`.

## 4. Fereastra de Settings

### 4.1 Structură

Fereastra de Settings are 960×720, cu un title bar macOS desenat (traffic lights) și stripul
de tab-uri centrat dedesubt:

```
┌──────────────────────────────────────────────────────────────┐
│ ● ● ●                                          Settings      │  ← title bar desenat
│                                                              │
│        ( ⚙ )   ( ▼ )   ( ⇄ )   ( ☁ )                        │  ← tab-uri, icon + label
│       General Modules Connectors Servers                     │
│  ─────────────────────────────────────────────────────────── │
│                                                              │
│  Indexing                                                    │
│  ▓▓▓▓▓▓▓░░░░░░░░  62%                                     │  ← bară 4dp + count
│  ─────────────────────────────────────────────────────────── │
│  Titlu opțiune                                      [switch] │
│  Descriere scurtă                                            │
│                                                              │
│  Quit Mirage                                                 │  ← acțiune de jos
└──────────────────────────────────────────────────────────────┘
```

### 4.2 Tab-uri de categorii

- Rand orizontal centrat, imediat sub title bar; fiecare tab este `icon` deasupra `label`.
- Tab activ: fundal `--color-selected-bg` pe icon, text `--color-text-primary`.
- Tab inactiv: text `--color-text-secondary`, fără fundal.
- Separator `MirageDivider()` sub strip, apoi conținutul tab-ului cu padding `spaceLg` pe
  orizontală și `spaceMd` pe verticală.
- Fără linie mov sub tab și fără carduri în jurul tab-urilor.

### 4.3 Rând standard pentru switch

```
┌────────────────────────────────────────────┐
│  Titlu opțiune                      [switch]│
│  Descriere opțiunii                         │
└────────────────────────────────────────────┘
```

- **Titlu**: `14px`, font-weight `500`, negru.
- **Descriere**: `12px`, `--color-text-secondary`, sub titlu.
- **Switch**: în dreapta, fără card în jurul rândului.
- Separator subțire între rânduri.

### 4.4 Rând standard pentru select

Varianta scurtă (etimă + select încap pe o linie):

```
┌────────────────────────────────────────────┐
│  Titlu opțiune                [Dropdown ▼]│
│  Descriere                                  │
└────────────────────────────────────────────┘
```

Varianta lungă (select mutat sub):

```
┌────────────────────────────────────────────┐
│  Titlu opțiune                             │
│  Descriere mai lungă care determină         │
│  layout-ul pe două rânduri                 │
│  [Dropdown ▼]                                │
└────────────────────────────────────────────┘
```

Regulă:
- Dacă titlul + descrierea + selectul încap confortabil pe o linie (max ~60% din lățime pentru text), rămân pe același rând.
- Dacă descrierea sau valoarea selectului sunt prea lungi, selectul coboară sub text.

### 4.5 Rând pentru progres descărcare modul

```
┌────────────────────────────────────────────┐
│  Module name                                 │
│  Downloading...  [====>    ]  34%  [Cancel] │
└────────────────────────────────────────────┘
```

## 5. Design Tokens compoziție

```kotlin
object MirageTokens {
    val colorBg = Color(0xFFFFFFFF)
    val colorTextPrimary = Color(0xFF000000)
    val colorTextSecondary = Color(0xFF6B7280)
    val colorBorder = Color(0xFFE5E7EB)
    val colorInputBorder = Color(0xFF000000)
    val colorSelectedBg = Color(0xFFEDE9FE)
    val colorKeyBg = Color(0xFFF3F4F6)
    val colorKeyText = Color(0xFF374151)
    val colorHoverBg = Color(0xFFF9FAFB)

    val spaceXs = 4.dp
    val spaceSm = 8.dp
    val spaceMd = 12.dp
    val spaceLg = 16.dp
    val spaceXl = 24.dp
    val inputPadding = 10.dp

    val radiusSm = 4.dp
    val radiusMd = 8.dp
    val radiusLg = 12.dp

    val textInput = 18.sp
    val textResultTitle = 14.sp
    val textResultMeta = 12.sp
    val textFooter = 12.sp
    val textSettingTitle = 14.sp
    val textSettingDesc = 12.sp
}
```

## 6. Note pentru implementare KMP Compose Desktop

- Folosiți `WindowDraggableArea` / `Modifier.pointerInput` pentru drag pe fundal.
- Iconițele aplicațiilor se iau prin API nativ al OS-ului; pe macOS se poate folosi `NSWorkspace.shared.icon(forFile:)` sau `NSWorkspace.shared.icon(forFileType:)`.
- Pentru fereastră preferați `ComposeWindow` fără decorații (undecorated) pentru look-ul de Spotlight.
- Pentru settings folosiți tot Compose Desktop, într-o fereastră separată cu dimensiuni fixe (ex. `640x480`).
- Rezultatele se actualizează în timp real la input; debounce ~100-150ms.

## 7. Decizii deschise

- Paleta pentru **dark mode** — se va defini după validarea light mode.
- Stilul exact al switch-ului (rounded vs. iOS-style) — se propune iOS-style minimal.
- Animarea apariției listei de rezultate — slide+fade 150ms.
