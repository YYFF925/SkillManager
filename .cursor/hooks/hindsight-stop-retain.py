#!/usr/bin/env python3
"""Cursor Cloud-compatible Hindsight auto-retain (stop hook).

Reads the stop-hook stdin JSON, optionally loads a transcript, and POSTs an
async retain to the Hindsight HTTP API. Fail-open: every error path exits 0.
Never prints secrets (tokens, Authorization headers, env values).

Cloud Agents support `stop` and do not support `sessionStart` / `sessionEnd`
on managed VMs, so this hook retains the full transcript on every stop when
credentials are present (no turn-window gate that would drop short sessions).
"""

from __future__ import annotations

import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

DEFAULT_API_URL = "https://hindsight.feiron.top"
DEFAULT_BANK_ID = "skillmanager"
USER_AGENT = "skillmanager-hindsight-stop-hook/1"
MAX_CONTENT_CHARS = 200_000
HTTP_TIMEOUT_SEC = 12


def _log(message: str) -> None:
    print(f"[hindsight-stop-retain] {message}", file=sys.stderr)


def _token() -> str:
    return (
        os.environ.get("HINDSIGHT_API_TOKEN")
        or os.environ.get("HINDSIGHT_API_KEY")
        or ""
    ).strip()


def _api_url() -> str:
    raw = (os.environ.get("HINDSIGHT_API_URL") or "").strip()
    return raw.rstrip("/")


def _bank_id() -> str:
    return (os.environ.get("HINDSIGHT_BANK_ID") or DEFAULT_BANK_ID).strip() or DEFAULT_BANK_ID


def _read_stdin() -> dict:
    raw = sys.stdin.read()
    if raw.startswith("\ufeff"):
        raw = raw.lstrip("\ufeff")
    raw = raw.strip()
    if not raw:
        return {}
    data = json.loads(raw)
    return data if isinstance(data, dict) else {}


def _normalize_blocks_to_text(content) -> str:
    if isinstance(content, str):
        return content
    if not isinstance(content, list):
        return str(content) if content else ""
    parts = []
    for block in content:
        if not isinstance(block, dict):
            continue
        btype = block.get("type")
        if btype == "text":
            text = block.get("text", "")
            if text:
                parts.append(text)
        elif btype == "tool_use":
            parts.append(f"[tool_use:{block.get('name', 'tool')}]")
        elif btype == "tool_result":
            parts.append("[tool_result]")
    return "\n".join(parts)


def _read_transcript(path: str) -> list[dict]:
    if not path or not os.path.isfile(path):
        return []
    messages: list[dict] = []
    try:
        with open(path, encoding="utf-8") as handle:
            for line in handle:
                line = line.strip()
                if not line:
                    continue
                try:
                    entry = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if not isinstance(entry, dict):
                    continue
                if entry.get("type") in ("user", "assistant"):
                    msg = entry.get("message", {})
                    if isinstance(msg, dict) and msg.get("role"):
                        messages.append(
                            {
                                "role": msg.get("role"),
                                "content": _normalize_blocks_to_text(msg.get("content")),
                            }
                        )
                    continue
                role = entry.get("role")
                if role in ("user", "assistant") and isinstance(entry.get("message"), dict):
                    messages.append(
                        {
                            "role": role,
                            "content": _normalize_blocks_to_text(entry["message"].get("content")),
                        }
                    )
                    continue
                if role in ("user", "assistant") and "content" in entry:
                    messages.append(
                        {
                            "role": role,
                            "content": _normalize_blocks_to_text(entry.get("content")),
                        }
                    )
    except OSError:
        return []
    return messages


def _format_transcript(messages: list[dict]) -> str:
    lines = []
    for msg in messages:
        role = (msg.get("role") or "").strip().lower()
        text = (msg.get("content") or "").strip()
        if not text or role not in ("user", "assistant"):
            continue
        label = "User" if role == "user" else "Assistant"
        lines.append(f"{label}: {text}")
    body = "\n\n".join(lines).strip()
    if len(body) > MAX_CONTENT_CHARS:
        body = body[-MAX_CONTENT_CHARS:]
    return body


def _retain(api_url: str, token: str, bank_id: str, content: str, document_id: str, session_id: str) -> None:
    path = f"/v1/default/banks/{urllib.parse.quote(bank_id, safe='')}/memories"
    url = f"{api_url}{path}"
    timestamp = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    body = {
        "items": [
            {
                "content": content,
                "context": "cursor-cloud",
                "document_id": document_id,
                "tags": ["source:chat", "harness:cursor-cloud", "skillmanager"],
                "strategy": "conversation",
                "observation_scopes": "shared",
                "metadata": {
                    "retained_at": timestamp,
                    "session_id": session_id,
                    "harness": "cursor-cloud",
                },
            }
        ],
        "async": True,
    }
    headers = {
        "Content-Type": "application/json",
        "User-Agent": USER_AGENT,
        "Authorization": f"Bearer {token}",
    }
    req = urllib.request.Request(url, data=json.dumps(body).encode("utf-8"), headers=headers, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=HTTP_TIMEOUT_SEC) as resp:
            _log(f"retain http={resp.status} bank={bank_id} chars={len(content)}")
    except urllib.error.HTTPError as exc:
        _log(f"retain http={exc.code} bank={bank_id}")
    except Exception:
        _log("retain failed")


def main() -> int:
    try:
        api_url = _api_url()
        token = _token()
        if not api_url or not token:
            _log("skipped: HINDSIGHT_API_URL and HINDSIGHT_API_TOKEN (or HINDSIGHT_API_KEY) not both set")
            return 0

        try:
            hook_input = _read_stdin()
        except (ValueError, json.JSONDecodeError):
            _log("skipped: invalid stdin json")
            return 0

        session_id = (
            hook_input.get("conversation_id")
            or hook_input.get("session_id")
            or os.environ.get("CURSOR_CONVERSATION_ID")
            or "unknown"
        )
        transcript_path = (
            hook_input.get("transcript_path")
            or os.environ.get("CURSOR_TRANSCRIPT_PATH")
            or ""
        )
        messages = _read_transcript(str(transcript_path))
        content = _format_transcript(messages)
        if not content:
            _log("skipped: empty transcript")
            return 0

        bank_id = _bank_id()
        document_id = f"conversation:{session_id}"
        if os.environ.get("HINDSIGHT_DRY_RUN") == "1":
            _log(f"dry-run bank={bank_id} chars={len(content)}")
            return 0
        _retain(api_url, token, bank_id, content, document_id, str(session_id))
        return 0
    except Exception:
        _log("unexpected error")
        return 0


if __name__ == "__main__":
    sys.exit(main())
