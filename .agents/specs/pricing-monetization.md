# Strategia de Monetizare — Mirage

## 1. Modelul comercial

Mirage folosește un model hibrid **Fair-Code / Freemium Local-First**, similar cu Obsidian, Syncthing sau Tailscale.

## 2. Tier-uri de preț

| Tier | Preț | Ce include? | Target User |
|------|------|-------------|-------------|
| Community / Local | 100% Gratuit (Open-Source) | Indexare 100% pe mașina locală (SSD). Fără limite de dimensiune a datelor locale. | Hobbyiști, studenți, utilizatori casual. |
| Pro / Cloud & Remote Vaults | $10 / Device (Licență pe viață) | Conectare la Remote Indexer (NAS), Dropbox, Google Drive, S3, Multi-Vault Sync & Licensing Key. | Freelanceri, creatori de conținut, ingineri. |
| Enterprise Team | $49 / Server | Container Remote Indexer cu suport RBAC și număr nelimitat de clienți conectați. | Echipe, agenții, companii. |

## 3. De ce funcționează prețul de $10 per-device

### 3.1 Psihologia utilizatorului open-source & privacy-conscious

- Utilizatorii local-first evită abonamentele lunare.
- O licență unică de $10 este percepută ca un gest de respect, nu ca o taxă corporativă.
- Plata one-time transmite sustenabilitate pe termen lung.

### 3.2 Delimitarea clară a valorii

- **Gratuit**: tot ce ține de utilizator și de datele sale locale.
- **Plătit**: conectarea la resurse remote și la un indexer partajat.

## 4. Implementarea licențierii fără server central

Pentru a păstra filozofia **Zero-Cloud & No-Account**:

1. **Achiziție**: utilizatorul cumpără licența ($10) prin LemonSqueezy sau Gumroad.
2. **Generare**: se generează un **Cryptographic License Key** semnat cu o cheie privată ED25519.
3. **Validare**: clientul Kotlin verifică offline dacă licența este validă folosind cheia publică împachetată în binar.
4. **Fără tracking**: nu există autentificare pe servere terțe, conturi de utilizator sau urmărire.

## 5. Flow de activare în UI

1. Utilizatorul apasă **Add Remote Vault** (NAS / Dropbox / Drive) în interfața Compose Desktop.
2. Se afișează un dialog cu **14 zile trial gratuit**.
3. După trial, se solicită introducerea **License Key**.
4. Licența se validează local; dacă este validă, funcțiile Pro sunt deblocate.

## 6. Cerințe de implementare

- [ ] Generare cheie ED25519 (privată în pipeline de release, publică înglobată în client).
- [ ] Format compact pentru License Key (ex: Base58 sau z-base-32).
- [ ] Validare offline în clientul KMP.
- [ ] Trial de 14 zile persistent local (nu se resetează la reinstalare fără mecanism suplimentar).
- [ ] Integrare cu LemonSqueezy / Gumroad pentru generarea automată a cheilor.
