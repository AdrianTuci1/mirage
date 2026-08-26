# ADR 007: Cloud / Self-Hosted Model and Removal of Offline Licensing

## Status

Accepted

## Context

Inițial, Mirage avea un model de monetizare bazat pe licențiere offline per-dispozitiv cu chei ED25519. Utilizatorii plăteau o dată $10 pentru funcțiile Pro (remote vaults).

Discuțiile ulterioare au clarificat că produsul va evolua către un model hibrid:

- **Local-only**: gratuit, open-source, rulează complet pe mașina utilizatorului.
- **Managed cloud**: utilizatorul plătește un abonament și noi procesăm datele pe serverele noastre.
- **Self-hosted**: utilizatorul rulează propriul container Remote Indexer.
- **Nu există licențiere offline**: gestionarea clienților și serverelor se face dintr-o platformă web separată.

## Decizie

1. **Eliminăm licențierea offline ED25519** și toate componentele asociate.
2. **Aplicația desktop nu distinge între managed și self-hosted**. Conectarea la un server se face printr-un cod.
3. **Aplicația desktop rulează AI local** (embeddings, vision, translator) pe ONNX, indiferent dacă este conectată sau nu la un server.
4. **Monetizarea se mută pe abonamente** (managed cloud / enterprise self-hosted).
5. **Platforma web de management** este în afara scope-ului acestui repository.

## Consecințe

- Se șterg task-urile T6.1–T6.4 din graf și fișierele asociate.
- Se adaugă un nou modul pentru modele AI locale.
- Flow-ul **Add Server** înlocuiește **Add Remote Vault**.
- Remote Vault URI rămâne suportat, dar este doar o formă de Server URI.
- Modelul este mai scalabil comercial și mai simplu pentru utilizator.

## Note

Acest ADR anulează orice decizie anterioară legată de licențierea offline.
