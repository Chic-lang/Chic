import sys
import unittest
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_DIR))

import codex_review  # noqa: E402


class CodexReviewJsonParsingTests(unittest.TestCase):
    def test_given_plain_json_then_extracts(self) -> None:
        text = '{"body":"x","comments":[]}'
        self.assertEqual(text, codex_review._extract_json_object(text))

    def test_given_fenced_json_then_extracts(self) -> None:
        text = "```json\n{\n  \"body\": \"x\",\n  \"comments\": []\n}\n```"
        extracted = codex_review._extract_json_object(text)
        self.assertIsNotNone(extracted)
        self.assertTrue(extracted.startswith("{"))
        self.assertTrue(extracted.endswith("}"))

    def test_given_wrapped_json_then_extracts(self) -> None:
        text = "Here you go:\n```json\n{\"body\":\"x\",\"comments\":[]}\n```\nThanks"
        extracted = codex_review._extract_json_object(text)
        self.assertEqual('{"body":"x","comments":[]}', extracted)

    def test_given_incomplete_json_then_returns_none(self) -> None:
        text = '{"body":"x"'
        self.assertIsNone(codex_review._try_parse_json(text))

    def test_given_malformed_json_then_returns_none(self) -> None:
        text = "```json\n{not json}\n```"
        self.assertIsNone(codex_review._try_parse_json(text))


if __name__ == "__main__":
    unittest.main()

