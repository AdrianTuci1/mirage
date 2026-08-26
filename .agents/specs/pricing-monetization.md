# Strategia de Monetizare — Mirage

## 1. Modelul comercial

Mirage folosește un model **freemium** în trei tier-uri, centrat pe locul unde rulează procesarea și cine administrează infrastructura.

- **Aplicația desktop și daemonul** sunt întotdeauna open-source și gratuite.
- **Procesarea** poate fi complet locală, pe un worker self-hosted sau în managed cloud.
- **Nu există licențiere offline**. Conectarea la servere se face prin User API Keys gestionate din Admin Web Console (self-hosted) sau Dashboard SaaS (managed).

## 2. Tier-uri de preț

| Tier | Infrastructură Worker | Interfață de Administrare | Cost |
|------|------------------------|---------------------------|------|
| **Community (Standalone)** | Local — rulează pe disc/NAS, fără worker remote. | Configurare directă în aplicație. | Gratuit / Open-Source |
| **Community (Self-Hosted)** | Worker lansat de utilizator pe VM/Docker propriu. | Admin Web Console integrată în worker (User API Keys). | Gratuit / Open-Source |
| **Managed Cloud (Pro/Ent)** | Cluster serverless orchestrat automat în cloud. | Dashboard SaaS Enterprise (SSO, RBAC). | Abonament per utilizator / volum compute |

## 3. Comparație caracteristici

| Feature | Standalone | Self-Hosted | Managed Cloud |
|---------|------------|-------------|---------------|
| Căutare locală vectorială | ✅ | ✅ | ✅ |
| DuckDB analytics local | ✅ | ✅ | ✅ |
| MCP / CLI | ✅ | ✅ | ✅ |
| Worker remote | ❌ | ✅ | ✅ |
| Admin console | ❌ | ✅ (local) | ✅ (cloud) |
| SSO / RBAC | ❌ | ❌ | ✅ |
| Support / SLA | ❌ | ❌ | ✅ |

## 4. De ce nu mai există licențiere offline

- **Flexibilitate**: utilizatorii pot alege local gratuit, self-hosted gratuit sau managed.
- **Simplificare**: aplicația desktop nu gestionează licențe, trial-uri sau chei criptografice.
- **Scalabilitate**: platforma web gestionează facturarea, accesul și monitorizarea.
- **Adecvare open-source**: workerul self-hosted rămâne 100% open-source.

## 5. Managementul cheilor

### Self-Hosted

1. Utilizatorul rulează `docker compose up` pentru worker.
2. Deschide Admin Web Console la `http://worker:8080/admin`.
3. Generează o User API Key pentru fiecare device.
4. În aplicația desktop apasă **Add Server**, introduce URL-ul workerului și codul (key).

### Managed Cloud

1. Utilizatorul se înregistrează în Dashboard SaaS.
2. Primește endpoint și User API Key.
3. Adaugă serverul în aplicație la fel ca self-hosted.

## 6. Condiții de acceptanță

- [ ] Utilizatorul poate rula Mirage 100% local fără cont.
- [ ] Utilizatorul poate rula propriul worker self-hosted cu Admin Web Console.
- [ ] Utilizatorul poate adăuga un server managed prin același flow.
- [ ] Aplicația desktop nu diferențiază între managed și self-hosted.
