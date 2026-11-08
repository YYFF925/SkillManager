---
name: hindsight-memory
description: When to search, capture, credit, and correct Hindsight long-term memory for SkillManager — especially on Cursor Cloud Agents where sessionStart injection does not run.
---

# Hindsight memory (Cloud-adapted)

This repo wires Hindsight without the laptop plugin (`npx @vectorize-io/hindsight-coding-agents install`). Memory is MCP tools plus an optional `stop` retain hook. There is no local `~/.hindsight` runtime on Cloud VMs.

Self-hosted API: `https://hindsight.feiron.top`. Project bank: `coding-agent::SkillManager` (the old lowercase `skillmanager` bank was merged into it and deleted on 2026-11-08).

Proven MCP URL for Cloud / Grok Bot: `https://hindsight.feiron.top/mcp/` (root multi-bank). Pass `bank_id=coding-agent::SkillManager` on every tool that takes a bank. A bank-scoped `{api}/mcp/{bankId}/` URL may also work; prefer the root URL.

HTTP MCP auth is a Bearer token from Infisical `/service/hindsight` key `MCP_BEARER_TOKEN`. Never commit it.

## What is automatic vs not

- **Cloud `sessionStart` / `sessionEnd`:** not available on managed Cloud Agent VMs. Do not wait for injected `<hindsight_memories>` or `.cursor/rules/hindsight-session.mdc`.
- **Optional `stop` hook:** if `HINDSIGHT_API_URL` and `HINDSIGHT_API_TOKEN` (or `HINDSIGHT_API_KEY`) are set, `.cursor/hooks/hindsight-stop-retain.py` may POST the transcript. Treat that as best-effort write-back, not a substitute for capturing durable decisions.
- **MCP tools:** Cloud Agents only see Hindsight if the root HTTP MCP (`https://hindsight.feiron.top/mcp/`) is added in the cursor.com/agents dashboard. Project `.cursor/mcp.json` is ignored on Cloud.

## Tool names (do not invent)

Call **only** tools that appear in the current tool list.

HTTP multi-bank MCP (dashboard / Grok Bot): `recall`, `retain`, `reflect` (and possibly `list_banks` / knowledge-page tools). Always pass `bank_id=coding-agent::SkillManager`.

coding-agents stdio (laptop plugin): `hindsight_search_knowledge_pages`, `hindsight_list_knowledge_pages`, `hindsight_read_knowledge_page`, `hindsight_reflect`, `hindsight_capture_initiative`, `hindsight_ingest_document`, `hindsight_sync_status`, `hindsight_diagnose`.

If neither family is present, say memory tools are unavailable and continue from git/docs/code.

## When to search

On any non-trivial task, search **before** reinventing project decisions, acceptance practice, vault/sync conventions, or prior user preferences:

1. `hindsight_search_knowledge_pages` if it exists, then read matching pages.
2. Otherwise `recall` with `bank_id=coding-agent::SkillManager`.
3. `reflect` / `hindsight_reflect` for why / exact decided values.

Empty hits are not proof of absence — retry a narrower query or report the miss.

Prefer knowledge pages for “what is true about this repo”; use reflect for synthesis.

## When to capture

- User says remember / store in Hindsight → `retain` (or `hindsight_ingest_document` / `hindsight_capture_initiative` if those tools exist).
- New initiative after the plan is agreed → `hindsight_capture_initiative` only if that tool exists; otherwise `retain` a short decision note.
- Do not store secrets, tokens, or content already authoritative in git.

## Credit and conflicts

When memory actually informs an answer, credit it (`From Hindsight memory: …`). Never credit unused memory.

When memory conflicts with current code or current `docs/` / `specs/`, prefer the repo, mention the mismatch, and correct the record if a write tool exists (`retain` a `Correction: …` note, or `hindsight_ingest_document` with what memory claimed, what is true now, and the evidence).

## Out of scope

Do not install npm packages into `$HOME`. Do not vendor the coding-agents runtime. Do not assume `~/.cursor/hooks.json` exists on Cloud.
