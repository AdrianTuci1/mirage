# Strategia de Monetizare — Mirage

## 1. Modelul comercial

Mirage folosește un model hibrid **Local-First + Managed Cloud / Self-Hosted**.

- **Aplicația desktop** este un motor de căutare semantică complet local: embeddings, vision și translator rulează pe mașina utilizatorului.
- **Conectarea la un server** (managed sau self-hosted) se face printr-un cod. Aplicația desktop nu știe dacă serverul este managed sau self-hosted.
- **Nu există licențiere offline**. Gestionarea clienților conectați la servere se face dintr-o platformă web separată, care nu face parte din acest repository.

## 2. Tier-uri de preț

| Tier | Preț | Ce include? | Target User |
|------|------|-------------|-------------|
| Community / Local | 100% Gratuit (Open-Source) | Indexare 100% pe mașina locală. Embedings, vision și translator local. Fără limite de dimensiune a datelor locale. | Hobbyiști, studenți, utilizatori casual. |
| Pro / Managed Cloud | Abonament lunar/anual | Server managed de noi care procesează datele utilizatorului. Utilizatorul primește un cod și sincronizează. | Freelanceri, creatori de conținut, ingineri. |
| Enterprise / Self-Hosted | Abonament per server | Container Remote Indexer self-hosted cu RBAC, suport multi-vault și număr nelimitat de clienți conectați. | Echipe, agenții, companii. |

## 3. De ce nu mai există licențiere offline

- **Flexibilitate**: utilizatorii pot alege între local gratuit, managed cloud sau self-hosted.
- **Simplificare**: aplicația desktop nu gestionează licențe, trial-uri sau chei criptografice.
- **Scalabilitate comercială**: platforma web gestionează clienții, facturarea și accesul la serverele managed.

## 4. Flow de conectare la un server

1. Utilizatorul apasă **Add Server** în interfața desktop.
2. Introduce **server URL** și **server code** (sau un Vault URI complet).
3. Clientul validează conexiunea printr-un request de handshake.
4. Dacă handshake-ul reușește, serverul este adăugat și sincronizarea începe.

## 5. Consecințe tehnice

- [ ] Eliminăm toate componentele de licențiere offline (ED25519, trial manager, license validator).
- [ ] Aplicația desktop suportă atât modul local-only, cât și modul conectat la orice server compatibil.
- [ ] Remote Indexer rămâne open-source și self-hostable.
- [ ] Platforma web de gestionare este în afara scope-ului acestui repository.

## 6. Condiții de acceptanță

- [ ] Utilizatorul poate rula Mirage 100% local fără cont sau licență.
- [ ] Utilizatorul poate adăuga un server self-hosted cu un cod.
- [ ] Utilizatorul poate adăuga un server managed cu un cod (contract identic).
- [ ] Aplicația desktop nu diferențiază între managed și self-hosted.
