#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.request
from dataclasses import dataclass
from typing import Any


def _read_diff(path: str, max_chars: int = 150_000) -> str:
    with open(path, "r", encoding="utf-8", errors="replace") as f:
        data = f.read()
    if len(data) <= max_chars:
        return data
    head = data[: max_chars // 2]
    tail = data[-max_chars // 2 :]
    return f"{head}\n\n[...diff truncated...]\n\n{tail}"


def _openai_chat_completion(api_key: str, model: str, prompt: str) -> str:
    url = "https://api.openai.com/v1/chat/completions"
    payload = {
        "model": model,
        "temperature": 0.2,
        "response_format": {"type": "json_object"},
        "messages": [
            {
                "role": "system",
                "content": (
                    "You are Codex, an expert software engineer doing GitHub PR reviews. "
                    "Be concise and specific. Prefer actionable bullets and point to likely file/logic areas. "
                    "Focus on correctness, tests, reliability, performance, security, and maintainability."
                ),
            },
            {"role": "user", "content": prompt},
        ],
    }
    req = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            body = resp.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as e:
        try:
            detail = e.read().decode("utf-8", errors="replace").strip()
        except Exception:
            detail = ""
        if detail:
            raise RuntimeError(f"OpenAI API request failed: HTTP {e.code} {e.reason}: {detail}") from e
        raise RuntimeError(f"OpenAI API request failed: HTTP {e.code} {e.reason}") from e
    except urllib.error.URLError as e:
        raise RuntimeError(f"OpenAI API request failed: {e.reason}") from e
    data = json.loads(body)
    return data["choices"][0]["message"]["content"]


@dataclass(frozen=True)
class _LineMap:
    path: str
    new_line_to_position: dict[int, int]
    annotated_patch: str


def _parse_unified_diff(diff_text: str) -> list[_LineMap]:
    # We map "new file line number" -> "diff position" for each file.
    # GitHub's review comment API expects `position` to be 1-based within the per-file patch,
    # where the first `@@ ... @@` line is position 1.
    file_maps: list[_LineMap] = []
    lines = diff_text.splitlines()
    i = 0

    def parse_hunk_header(header: str) -> tuple[int, int]:
        # @@ -oldStart,oldCount +newStart,newCount @@
        # counts may be omitted
        # returns (oldStart, newStart)
        try:
            at = header.split("@@")[1].strip()
            parts = at.split(" ")
            old_part = parts[0]  # -a,b
            new_part = parts[1]  # +c,d
            old_start = int(old_part.split(",")[0][1:])
            new_start = int(new_part.split(",")[0][1:])
            return old_start, new_start
        except Exception:
            return 0, 0

    while i < len(lines):
        line = lines[i]
        if not line.startswith("diff --git "):
            i += 1
            continue

        # Example: diff --git a/src/Foo.cs b/src/Foo.cs
        parts = line.split(" ")
        if len(parts) < 4:
            i += 1
            continue

        b_path = parts[3]
        if not b_path.startswith("b/"):
            i += 1
            continue

        path = b_path[2:]
        i += 1

        patch_started = False
        position = 0
        old_line_no = 0
        new_line_no = 0
        new_line_to_position: dict[int, int] = {}
        annotated_lines: list[str] = [f"FILE: {path}"]

        while i < len(lines) and not lines[i].startswith("diff --git "):
            current = lines[i]

            if current.startswith("@@ "):
                patch_started = True
                position += 1
                old_line_no, new_line_no = parse_hunk_header(current)
                annotated_lines.append(current)
                i += 1
                continue

            if not patch_started:
                i += 1
                continue

            position += 1
            if current.startswith("+") and not current.startswith("+++"):
                new_line_to_position[new_line_no] = position
                annotated_lines.append(f"{new_line_no:>6} +{current[1:]}")
                new_line_no += 1
            elif current.startswith(" ") and not current.startswith("+++"):
                new_line_to_position[new_line_no] = position
                annotated_lines.append(f"{new_line_no:>6}  {current[1:]}")
                old_line_no += 1
                new_line_no += 1
            elif current.startswith("-") and not current.startswith("---"):
                annotated_lines.append(f"{old_line_no:>6} -{current[1:]}")
                old_line_no += 1
            else:
                annotated_lines.append(current)
            i += 1

        if patch_started:
            file_maps.append(
                _LineMap(
                    path=path,
                    new_line_to_position=new_line_to_position,
                    annotated_patch="\n".join(annotated_lines),
                )
            )

    return file_maps


def _truncate(text: str, max_chars: int) -> str:
    if len(text) <= max_chars:
        return text
    head = text[: max_chars // 2]
    tail = text[-max_chars // 2 :]
    return f"{head}\n\n[...truncated...]\n\n{tail}"


def _extract_json_object(text: str) -> str | None:
    text = text.strip()
    if not text:
        return None

    if text.startswith("```"):
        # Common model output shape:
        # ```json
        # { ... }
        # ```
        first_newline = text.find("\n")
        if first_newline != -1:
            text = text[first_newline + 1 :]
        if text.endswith("```"):
            text = text[: -3]
        text = text.strip()

    if text.startswith("{") and text.endswith("}"):
        return text

    # Best-effort extraction if the model wrapped JSON with extra text.
    first = text.find("{")
    last = text.rfind("}")
    if first != -1 and last != -1 and last > first:
        return text[first : last + 1]

    return None


def _try_parse_json(text: str) -> Any | None:
    extracted = _extract_json_object(text)
    if extracted is None:
        return None
    try:
        return json.loads(extracted)
    except Exception:
        return None


def _redact_secrets(text: str, secrets: list[str]) -> str:
    redacted = text
    for secret in secrets:
        if secret:
            redacted = redacted.replace(secret, "[REDACTED]")
    if "Bearer " in redacted:
        # Best-effort header redaction.
        redacted = redacted.replace("Bearer ", "Bearer [REDACTED]")
    return redacted


def main(argv: list[str] | None = None) -> int:
    argv = argv if argv is not None else sys.argv
    if len(argv) != 2:
        print("Usage: codex_review.py <pr.diff>", file=sys.stderr)
        return 2

    api_key = os.environ.get("OPENAI_API_KEY", "").strip()
    if not api_key:
        print("Missing OPENAI_API_KEY secret; cannot generate Codex review.", file=sys.stderr)
        return 1

    model = os.environ.get("OPENAI_MODEL", "gpt-4o-mini").strip() or "gpt-4o-mini"
    diff_text = _read_diff(argv[1])
    file_maps = _parse_unified_diff(diff_text)

    annotated = "\n\n".join(m.annotated_patch for m in file_maps)
    annotated = _truncate(annotated, max_chars=150_000)

    prompt = (
        "You are Codex, an expert software engineer doing GitHub PR reviews.\n"
        "Return a single JSON object with:\n"
        "- body: string (markdown) containing: Summary, High-risk issues, Test gaps, Cleanup suggestions\n"
        "- comments: array of up to 10 objects, each with:\n"
        "  - path: string (exact file path from FILE headers)\n"
        "  - newLine: integer (a RIGHT-side line number shown in the annotated diff)\n"
        "  - body: string (1-3 bullet points, actionable)\n"
        "\n"
        "Only use path/newLine values that exist in the annotated diff below. Prefer commenting on changed lines.\n"
        "Do not include any secrets.\n\n"
        "Annotated diff:\n"
        f"{annotated}"
    )

    try:
        raw = _openai_chat_completion(api_key, model, prompt)
        data = _try_parse_json(raw)
        if not isinstance(data, dict):
            raise RuntimeError("Codex review output was not valid JSON.")

        body = (data.get("body") or "").strip()
        comments_in = data.get("comments") or []
        if not isinstance(comments_in, list):
            comments_in = []

        comments_out: list[dict[str, Any]] = []
        for item in comments_in[:10]:
            if not isinstance(item, dict):
                continue
            path = (item.get("path") or "").strip()
            new_line = item.get("newLine")
            comment_body = (item.get("body") or "").strip()
            if not path or not isinstance(new_line, int) or not comment_body:
                continue

            line_map = next((m for m in file_maps if m.path == path), None)
            if line_map is None:
                continue
            pos = line_map.new_line_to_position.get(new_line)
            if pos is None:
                continue

            comments_out.append(
                {
                    "path": path,
                    "position": pos,
                    "body": comment_body,
                }
            )

        marker = "<!-- codex-review-inline -->"
        review_body = "\n".join(
            [
                marker,
                "# Codex review (automated)",
                "",
                body or "_No review body produced._",
            ]
        )
        out = {"body": review_body, "comments": comments_out}
        print(json.dumps(out, indent=2))
        return 0

    except Exception as e:
        secrets = [api_key]
        msg = _redact_secrets(str(e), secrets)
        # Emit a safe payload so the workflow can still create a minimal review.
        marker = "<!-- codex-review-inline -->"
        out = {
            "body": f"{marker}\n# Codex review (automated)\n\nFailed to generate review: `{msg}`",
            "comments": [],
        }
        print(json.dumps(out, indent=2))
        return 0


if __name__ == "__main__":
    raise SystemExit(main())

