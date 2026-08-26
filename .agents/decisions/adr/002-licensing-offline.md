# ADR 002: Licențiere offline cu ED25519

## Status

Pending

## Context

Mirage promovează filozofia **No-Account / Zero-Cloud**. Nu putem avea un server central de licențiere care să ceară autentificare.

## Opțiuni evaluate

### Opțiunea A: Server de licențiere online

**Pro:**
- Revocare ușoară a licențelor.
- Statistici de utilizare.

**Contra:**
- Contrazice filozofia no-account.
- Single point of failure.
- Necesită costuri de operare.

### Opțiunea B: Licență criptografică offline (ED25519)

**Pro:**
- Fără server central.
- Validare instant în client.
- Rezistent la spoofing dacă cheia privată este păstrată secretă.

**Contra:**
- Revocarea este dificilă fără mecanisme suplimentare.
- Cheia publică înglobată poate fi extrasa din binar (acceptable risk).

## Decizie propusă

**Opțiunea B**: fiecare licență Pro este un token semnat cu ED25519. Clientul conține cheia publică și validează semnătura local.

## Consecințe

- Cheia privată rămâne în pipeline-ul de release.
- Formatul licenței trebuie să fie compact și ușor de introdus de utilizator.
- Trial-ul de 14 zile este gestionat local.

## Note

Pentru revocare la scară largă se poate adăuga în viitor o listă de licențe invalidate (CRL) distribuită opțional cu update-uri.
