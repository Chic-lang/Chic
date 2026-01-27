import io
import json
import sys
import unittest
import urllib.error
from pathlib import Path
from unittest import mock


SCRIPT_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_DIR))

import codex_review  # noqa: E402


class CodexReviewErrorHandlingTests(unittest.TestCase):
    def test_given_http_error_then_raises_runtime_error_with_status(self) -> None:
        error_body = json.dumps({"error": {"message": "bad key"}}).encode("utf-8")
        fp = io.BytesIO(error_body)
        http_error = urllib.error.HTTPError(
            url="https://api.openai.com/v1/chat/completions",
            code=401,
            msg="Unauthorized",
            hdrs=None,
            fp=fp,
        )

        with mock.patch("urllib.request.urlopen", side_effect=http_error):
            with self.assertRaises(RuntimeError) as ctx:
                codex_review._openai_chat_completion("secret", "model", "prompt")

        self.assertIn("HTTP 401", str(ctx.exception))
        self.assertIn("bad key", str(ctx.exception))

