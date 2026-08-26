# ADR 010: Model Context Protocol (MCP) Support

## Status

Accepted

## Context

Agenții AI (Claude, Octomus, etc.) au nevoie de acces structurat la datele utilizatorului. Protocolul MCP este standardul emergent pentru conectarea LLM-urilor la date și tool-uri locale.

## Decizie

Implementăm un server MCP în CLI (`mirage mcp serve`) care se conectează la daemon prin IPC.

- Transport: stdio.
- Tools: `search`, `query`, `index_path`, `status`.
- Resources: fișiere indexate (read-only).
- Prompts: exemple de interogări.

## Consecințe

- Clienții MCP pot căuta în datele locale fără să acceseze direct daemonul.
- CLI devine mai complex, dar rămâne binar mic.
- Securitate: MCP serverul rulează în sesiunea utilizatorului și se conectează doar la daemon local.

## Note

MCP este opțional. Utilizatorul îl poate activa doar dacă dorește integrare cu agenți AI.
