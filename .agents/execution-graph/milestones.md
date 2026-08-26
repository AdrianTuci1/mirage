# Milestones — Mirage

## M0: Setup repo & tooling

**Obiectiv:** Proiectul este inițializat, structura `.agents` există, repo-ul Git este funcțional.

- [x] Inițializare repo și foldere
- [ ] Configurează build tool (Gradle pentru KMP, Docker pentru indexer)
- [ ] Adaugă CI minimal (format, lint)
- [ ] Setup test harness

**Livrabil:** Repo-ul poate fi clonat și build-urile pornesc fără erori.

## M1: Remote indexer skeleton & LanceDB integration

**Obiectiv:** Containerul remote indexer poate scana surse și stoca vectori în LanceDB.

- [ ] Dockerfile și docker-compose.yml
- [ ] LanceDB schema și inserare vectori
- [ ] ONNX Runtime inferență
- [ ] Storage connectors (local, NAS, S3, Dropbox, Google Drive)
- [ ] Pipeline de indexare incrementală

**Livrabil:** `docker compose up` pornește indexerul și poate indexa un folder local de test.

## M2: Delta sync endpoint (HTTP2/gRPC)

**Obiectiv:** Clienții pot descărca doar diferența de index de la server.

- [ ] Alegere protocol (gRPC vs HTTP2 streaming)
- [ ] Endpoint `/sync/delta` cu autentificare passkey
- [ ] Generare fișiere `.lance` delta

**Livrabil:** Un client de test poate descărca delta după o versiune dată.

## M3: KMP client engine & Vault URI parser

**Obiectiv:** Clientul desktop poate parse Vault URI și sincroniza indexul local.

- [ ] KMP project skeleton
- [ ] Vault URI parser
- [ ] RemoteVaultManager
- [ ] Integrare LanceDB local

**Livrabil:** Aplicația desktop se conectează la un indexer și descarcă indexul.

## M4: VFS adapters (local, Dropbox, Google Drive, SMB)

**Obiectiv:** Fișierele se deschid direct din sursă, fără proxy prin server.

- [ ] Interfață VfsAdapter
- [ ] Adaptoare pentru local, Dropbox, Google Drive, SMB
- [ ] Cache de preview/thumbnail

**Livrabil:** Utilizatorul poate deschide un fișier din orice sursă conectată.

## M5: Search UI in Compose Desktop

**Obiectiv:** Interfața de căutare funcționează și arată bine.

- [ ] Ecran de căutare cu rezultate
- [ ] Preview thumbnail
- [ ] Flow Add Remote Vault

**Livrabil:** Utilizatorul poate căuta și previzualiza fișiere.

## M6: Offline licensing (ED25519)

**Obiectiv:** Licențierea Pro este implementată fără server central.

- [ ] Chei ED25519 și format License Key
- [ ] Validare offline în client
- [ ] Trial de 14 zile

**Livrabil:** Funcțiile Pro se deblochează doar cu licență validă sau în trial.

## M7: Packaging & release

**Obiectiv:** Proiectul este gata de distribuție.

- [ ] Docker image multi-arch
- [ ] Desktop installers (MSI, DMG, DEB)
- [ ] Smoke tests end-to-end
- [ ] Documentație finală

**Livrabil:** Release public pe GitHub / Docker Hub / Gumroad.
