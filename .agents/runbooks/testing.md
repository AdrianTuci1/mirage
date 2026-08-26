# Testing Strategy — Mirage

## Filozofie

Pentru că Mirage nu este o aplicație web, nu putem folosi Playwright. Echivalentele pentru un client desktop KMP + un server Python sunt:

- **Unit tests** — logica pură (parse URI, validare, transformări).
- **Integration tests** — apeluri HTTP cu mock (Ktor MockEngine pentru client, FastAPI TestClient pentru server).
- **UI tests** — Compose Desktop `runComposeUiTest` (echivalentul cel mai apropiat de Playwright pentru interfață).
- **E2E tests** — pornesc aplicația reală și serverul real într-un mediu temporar.

## Stack de testare

### Remote Indexer (Python)

| Tip | Tool | Comandă |
|-----|------|---------|
| Unit/Integration | pytest + FastAPI TestClient | `pytest -v` |
| HTTP mocking | FastAPI TestClient | — |
| LanceDB izolat | pytest tmp_path + monkeypatch | — |

### KMP Client (Kotlin)

| Tip | Tool | Comandă |
|-----|------|---------|
| Unit (common) | kotlin-test | `./gradlew jvmTest` |
| Coroutine tests | kotlinx-coroutines-test | — |
| HTTP mocking | Ktor MockEngine | — |
| UI Desktop | Compose `ui-test` + `runComposeUiTest` | `./gradlew jvmTest` |

## Cum rulezi testele

### Remote Indexer

```bash
cd src/remote-indexer
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
pytest -v
```

Teste curente:
- `test_health_and_sync.py` — endpoint-uri `/health` și `/sync/delta`.
- `test_lancedb_schema.py` — schema și inserare de record în LanceDB.

### KMP Client

```bash
cd src/client-kmp
export JAVA_HOME=/path/to/jdk-21
./gradlew jvmTest
```

Teste curente:
- `VaultUriParserTest` — parsing valid și invalid de Vault URI.
- `RemoteVaultManagerTest` — request URL, auth header, propagare erori (MockEngine).
- `SearchScreenUiTest` — verificare că ecranul de căutare randează textul așteptat.

## Echivalent Playwright pentru desktop

| Playwright | KMP / Compose Desktop |
|------------|----------------------|
| `page.goto()` | `runComposeUiTest { setContent { ... } }` |
| `page.locator()` | `onNodeWithText(...)`, `onNodeWithTag(...)` |
| `expect(...).toBeVisible()` | `.assertIsDisplayed()` |
| `page.click()` | `.performClick()` |
| API mocking | Ktor MockEngine |

## E2E în viitor

Pentru E2E complet vom avea nevoie de:
1. Pornirea Remote Indexer într-un container Docker cu date de test.
2. Pornirea aplicației desktop și conectarea la indexer.
3. Automatizare UI cu `Compose UiTest` sau `java.awt.Robot`.
4. Curățarea resurselor după test.

Aceasta este țintată pentru milestone M7.

## Reguli pentru agenți

1. Adaugă teste pentru orice endpoint API nou în `src/remote-indexer/tests/`.
2. Adaugă teste pentru orice clasă utilitară nouă în `src/commonTest/` sau `src/jvmTest/`.
3. Mock-ează apelurile externe (Dropbox, Google Drive); nu face request-uri reale în teste.
4. Rulează `pytest -v` și `./gradlew jvmTest` înainte de fiecare commit.
5. Documentează orice test care necesită setup special în acest fișier.
