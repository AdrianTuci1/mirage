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

| Token | Valoare | Utilizare |
|-------|---------|-----------|
| `--color-bg` | `#FFFFFF` | Fundal fereastră principală și settings |
| `--color-text-primary` | `#000000` | Text principal, titluri, input |
| `--color-text-secondary` | `#6B7280` | Descrieri, placeholder, metadata |
| `--color-border` | `#E5E7EB` | Chenar exterior input, separator, `<tab>` box |
| `--color-input-border` | `#000000` | Chenar interior input (negru, 1px) |
| `--color-selected-bg` | `#EDE9FE` / `#DDD6FE` | Fundal element selectat (mov deschis / desaturat) |
| `--color-selected-text` | `#000000` | Text pe element selectat |
| `--color-key-bg` | `#F3F4F6` | Fundal pentru taste afișate (`<tab>`, `<x>`, `<option>`) |
| `--color-key-text` | `#374151` | Text taste afișate |
| `--color-hover-bg` | `#F9FAFB` | Hover subtil pe elemente neselectate |

> Notă: toate culorile sunt valori orientative pentru light mode. Dark mode va fi definit ulterior prin înversarea luminii păstrând accentul mov.

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

```
┌────────────────────────────────────────────┐
│  [icon] General  │  [icon] Modules  │ ...   │  ← Tab-uri categorii
│  ─────────────────────────────────────────  │
│                                              │
│  Titlu opțiune                             [●] │  ← Rând switch
│  Descriere scurtă                            │
│  ─────────────────────────────────────────  │
│  Titlu opțiune                               │
│  Descriere mai lungă care justifică         │
│  [Select / dropdown]                         │  ← Rând select sub text
│                                              │
└────────────────────────────────────────────┘
```

### 4.2 Tab-uri de categorii

- Rand orizontal în partea de sus.
- Fiecare tab: `icon + descriere`, separate prin `|` sau spațiu.
- Tab activ: text negru + linie subțire mov dedesubt.
- Tab inactiv: text `--color-text-secondary`.
- Fără carduri sau fundal pentru tab-uri.

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
