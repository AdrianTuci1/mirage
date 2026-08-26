# ADR 004: UI Paradigm — Spotlight/Raycast-style Global Search

## Status

Accepted

## Context

Inițial, Mirage era descris ca o aplicație desktop clasică cu fereastră permanentă. Discuția ulterioară a clarificat că produsul final trebuie să funcționeze ca un **launcher global** similar cu Spotlight pe macOS sau Raycast: o scurtătură globală deschide o fereastră flotantă de căutare, utilizatorul tastează un query, primește rezultate, acționează și fereastra se ascunde.

## Decizie

Mirage va fi un **global search launcher** pentru desktop:

- **Global hotkey** (`Ctrl + Space` pe Windows/Linux, `Cmd + Space` pe macOS; configurabil) pentru toggle rapid.
- **Fereastră flotantă** compactă, centrată pe ecranul activ (cel cu cursorul mouse-ului), fără decorațiuni, afișată/ascunsă la hotkey.
- **System tray icon** pentru acces la Settings / Quit.
- **Clipboard history** indexabil și căutabil (feature opțional, activabil din setări).

## Consecințe

- Nu mai avem o fereastră principală persistentă. Aplicația rulează în background și apare doar la invocare.
- UI-ul trebuie să fie extrem de rapid: timp de afișare < 100ms.
- Fereastra trebuie să rețină starea între apeluri (query, poziție scroll).
- Este nevoie de librării native pentru global hotkey (JNativeHook) și de API-uri AWT pentru system tray/clipboard.
- Design-ul trebuie să funcționeze pe macOS, Windows și Linux cu aceeași logică, dar adaptări minime pentru taste (Cmd vs Ctrl).

## Tehnologii

- **Compose Multiplatform Desktop**: UI.
- **JNativeHook**: global hotkeys.
- **AWT SystemTray**: tray icon.
- **AWT Clipboard**: clipboard history.

## Note

Pe macOS, JNativeHook necesită permisiunea Accessibility. Documentația de instalare va include pașii.
