Cloud Agents recreate Hindsight auto-retain with a repo `stop` hook. They do not load `~/.cursor` and do not run `sessionStart` / `sessionEnd` on managed VMs.

## What runs

`.cursor/hooks.json` registers only `stop` → `python3 .cursor/hooks/hindsight-stop-retain.py`.

The script reads Cursor's hook stdin JSON (`conversation_id`, `transcript_path` when present), formats the transcript, and POSTs an async retain to `{HINDSIGHT_API_URL}/v1/default/banks/skillmanager/memories` (`HINDSIGHT_BANK_ID` overrides the bank). It always exits 0. It never prints tokens or Authorization headers.

`beforeSubmitPrompt` is intentionally not registered (latency on every prompt). Do not add `sessionStart` for Cloud.

## Required environment (Cloud dashboard / VM secrets)

Set these on the Cloud Agent environment (or locally). **Do not commit values.**

The Bearer token lives in Infisical at `/service/hindsight` key `MCP_BEARER_TOKEN`. Inject that secret into the VM as `HINDSIGHT_API_TOKEN` (or `HINDSIGHT_API_KEY`). Use the same Bearer token in the cursor.com/agents MCP dashboard Authorization header.

| Variable | Required | Purpose |
|---|---|---|
| `HINDSIGHT_API_URL` | yes | REST API base, no `/mcp` suffix. Self-hosted: `https://hindsight.feiron.top` |
| `HINDSIGHT_API_TOKEN` | yes* | Bearer token (from Infisical `MCP_BEARER_TOKEN`) |
| `HINDSIGHT_API_KEY` | yes* | Alias if `HINDSIGHT_API_TOKEN` is unset |
| `HINDSIGHT_BANK_ID` | no | Default `skillmanager` (lowercase; do not use `coding-agent::SkillManager`) |
| `HINDSIGHT_DRY_RUN` | no | `1` skips the POST (for local checks) |

\* One of `HINDSIGHT_API_TOKEN` or `HINDSIGHT_API_KEY` must be set, or the hook no-ops.

Without those env vars the hook is a silent no-op. That is expected until Cloud secrets are configured.

## MCP tools are separate

This hook only writes memory via the REST retain API. On-demand tools require an HTTP MCP server in the **cursor.com/agents MCP dashboard**. Cloud Agents ignore repo `.cursor/mcp.json`.

Proven MCP URL (root multi-bank, what Grok Bot already uses):

`https://hindsight.feiron.top/mcp/`

Pass `bank_id=skillmanager` on tool calls. A bank-scoped `{api}/mcp/{bankId}/` URL may also work for editor-style tools; prefer the root URL. See `.cursor/mcp.json.example` (placeholder only — no token in git).
