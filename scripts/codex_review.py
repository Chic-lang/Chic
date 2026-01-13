#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import sys
import textwrap
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class PullRequestContext:
    owner: str
    repo: str
    number: int
    base_sha: str
    head_sha: str
    html_url: str


def _env(name: str) -> str:
    value = os.getenv(name)
    if not value:
        raise RuntimeError(f"Missing required env var: {name}")
    return value


def _github_api_request(
    token: str,
    url: str,
    *,
    method: str = "GET",
    accept: str = "application/vnd.github+json",
    body: dict[str, Any] | None = None,
) -> Any:
    headers = {
        "Authorization": f"Bearer {token}",
        "Accept": accept,
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "chic-codex-review",
    }
    data = None
    if body is not None:
        data = json.dumps(body).encode("utf-8")
        headers["Content-Type"] = "application/json"

    request = urllib.request.Request(url, data=data, headers=headers, method=method)
    with urllib.request.urlopen(request, timeout=60) as response:
        raw = response.read()
        if not raw:
            return None
        return json.loads(raw.decode("utf-8"))


def _openai_responses_request(api_key: str, model: str, input_text: str) -> str:
    url = os.getenv("OPENAI_BASE_URL", "https://api.openai.com/v1").rstrip("/") + "/responses"
    payload = {
        "model": model,
        "input": [
            {
                "role": "system",
                "content": [
                    {
                        "type": "input_text",
                        "text": (
                            "You are a senior engineer reviewing a pull request in the Chic programming language "
                            "repository. Provide a concise, high-signal review focused on correctness, security, "
                            "maintainability, and documentation. Prefer actionable bullets. If you mention a file, "
                            "include the path. Do not invent context that is not in the diff."
                        ),
                    }
                ],
            },
            {
                "role": "user",
                "content": [{"type": "input_text", "text": input_text}],
            },
        ],
        "max_output_tokens": 900,
    }

    request = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
        method="POST",
    )

    with urllib.request.urlopen(request, timeout=120) as response:
        data = json.loads(response.read().decode("utf-8"))

    if isinstance(data, dict) and "output_text" in data and isinstance(data["output_text"], str):
        return data["output_text"].strip()

    output_parts: list[str] = []
    for item in data.get("output", []):
        if item.get("type") != "message":
            continue
        for content in item.get("content", []):
            if content.get("type") == "output_text" and isinstance(content.get("text"), str):
                output_parts.append(content["text"])

    text = "\n".join(output_parts).strip()
    if not text:
        raise RuntimeError("OpenAI response contained no output text")
    return text


def _load_pr_context() -> PullRequestContext:
    event_path = _env("GITHUB_EVENT_PATH")
    with open(event_path, "r", encoding="utf-8") as handle:
        event = json.load(handle)

    pr = event.get("pull_request", {})
    repo = event.get("repository", {})
    owner = repo.get("owner", {}).get("login", "")
    name = repo.get("name", "")
    number = int(pr.get("number"))
    base_sha = pr.get("base", {}).get("sha", "")
    head_sha = pr.get("head", {}).get("sha", "")
    html_url = pr.get("html_url", "")

    if not owner or not name or not base_sha or not head_sha or not html_url:
        raise RuntimeError("Missing expected pull_request_target event fields")

    return PullRequestContext(owner=owner, repo=name, number=number, base_sha=base_sha, head_sha=head_sha, html_url=html_url)


def _truncate(text: str, max_chars: int) -> str:
    if len(text) <= max_chars:
        return text
    return text[: max_chars - 200] + "\n…(truncated)…\n"


def _format_review_input(pr_ctx: PullRequestContext, github_token: str) -> str:
    pr = _github_api_request(
        github_token,
        f"https://api.github.com/repos/{pr_ctx.owner}/{pr_ctx.repo}/pulls/{pr_ctx.number}",
    )

    title = pr.get("title", "")
    body = pr.get("body", "") or ""
    author = (pr.get("user") or {}).get("login", "")

    files: list[dict[str, Any]] = []
    page = 1
    while True:
        batch = _github_api_request(
            github_token,
            f"https://api.github.com/repos/{pr_ctx.owner}/{pr_ctx.repo}/pulls/{pr_ctx.number}/files?per_page=100&page={page}",
        )
        if not batch:
            break
        files.extend(batch)
        if len(batch) < 100:
            break
        page += 1

    changed_files = []
    for f in files:
        filename = f.get("filename", "")
        status = f.get("status", "")
        patch = f.get("patch")
        additions = f.get("additions", 0)
        deletions = f.get("deletions", 0)

        if not filename:
            continue

        entry = [f"File: {filename} ({status}, +{additions}/-{deletions})"]
        if isinstance(patch, str) and patch.strip():
            entry.append(_truncate(patch, 14_000))
        else:
            entry.append("(no patch available; likely binary or too large)")
        changed_files.append("\n".join(entry))

    summary = textwrap.dedent(
        f"""\
        PR: {pr_ctx.html_url}
        Title: {title}
        Author: {author}

        Description (from PR body):
        {_truncate(body, 6_000) if body else "(none)"}

        Diff (file patches):
        """
    ).rstrip()

    return summary + "\n\n" + "\n\n---\n\n".join(changed_files[:25])


def _find_existing_bot_comment(github_token: str, pr_ctx: PullRequestContext) -> int | None:
    page = 1
    marker = "<!-- chic-codex-review -->"
    while True:
        comments = _github_api_request(
            github_token,
            f"https://api.github.com/repos/{pr_ctx.owner}/{pr_ctx.repo}/issues/{pr_ctx.number}/comments?per_page=100&page={page}",
        )
        if not comments:
            return None
        for comment in comments:
            body = comment.get("body") or ""
            if marker in body:
                return int(comment.get("id"))
        if len(comments) < 100:
            return None
        page += 1


def _upsert_comment(github_token: str, pr_ctx: PullRequestContext, body: str) -> None:
    existing_id = _find_existing_bot_comment(github_token, pr_ctx)
    if existing_id is None:
        _github_api_request(
            github_token,
            f"https://api.github.com/repos/{pr_ctx.owner}/{pr_ctx.repo}/issues/{pr_ctx.number}/comments",
            method="POST",
            body={"body": body},
        )
        return

    _github_api_request(
        github_token,
        f"https://api.github.com/repos/{pr_ctx.owner}/{pr_ctx.repo}/issues/comments/{existing_id}",
        method="PATCH",
        body={"body": body},
    )


def main() -> int:
    pr_ctx = _load_pr_context()
    github_token = _env("GITHUB_TOKEN")

    openai_api_key = os.getenv("OPENAI_API_KEY", "")
    if not openai_api_key:
        raise RuntimeError("OPENAI_API_KEY is not set (configure a GitHub Actions secret)")

    model = os.getenv("OPENAI_MODEL") or "gpt-4o-mini"

    review_input = _format_review_input(pr_ctx, github_token)

    started = time.time()
    try:
        review = _openai_responses_request(openai_api_key, model, review_input)
    except urllib.error.HTTPError as e:
        err = e.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"OpenAI API request failed: HTTP {e.code}: {err}") from e

    elapsed = int(time.time() - started)
    marker = "<!-- chic-codex-review -->"
    comment = "\n".join(
        [
            marker,
            "## Codex review (automated)",
            "",
            f"_Model: `{model}` · Generated in ~{elapsed}s_",
            "",
            review.strip(),
            "",
            "If anything here is unclear or off, reply with what you want changed and I can re-review.",
        ]
    ).strip()

    _upsert_comment(github_token, pr_ctx, comment)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as e:
        print(f"codex_review.py: {e}", file=sys.stderr)
        raise
