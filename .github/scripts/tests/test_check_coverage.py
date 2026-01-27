import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_DIR))

import check_coverage  # noqa: E402


def _write(path: Path, text: str) -> None:
    path.write_text(text, encoding="utf-8")


def _cobertura_xml(lines_covered: int, lines_valid: int) -> str:
    return f"""<?xml version="1.0"?>
<coverage lines-covered="{lines_covered}" lines-valid="{lines_valid}">
  <packages>
    <package name="p" />
  </packages>
</coverage>
"""


def _cobertura_xml_with_line_rate(line_rate: float) -> str:
    return f"""<?xml version="1.0"?>
<coverage line-rate="{line_rate}">
  <packages>
    <package name="p" />
  </packages>
</coverage>
"""


class CheckCoverageTests(unittest.TestCase):
    def test_given_invalid_args_then_returns_2(self) -> None:
        self.assertEqual(2, check_coverage.main(["check_coverage.py"]))

    def test_given_invalid_threshold_then_returns_2(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            xml_path = Path(tmp) / "c.xml"
            _write(xml_path, _cobertura_xml(95, 100))
            self.assertEqual(2, check_coverage.main(["check_coverage.py", str(xml_path), "nope"]))

    def test_given_missing_file_then_returns_2(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            xml_path = Path(tmp) / "missing.xml"
            self.assertEqual(2, check_coverage.main(["check_coverage.py", str(xml_path), "0.95"]))

    def test_given_malformed_xml_then_returns_2(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            xml_path = Path(tmp) / "bad.xml"
            _write(xml_path, "<coverage>")
            self.assertEqual(2, check_coverage.main(["check_coverage.py", str(xml_path), "0.95"]))

    def test_given_empty_file_then_returns_2(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            xml_path = Path(tmp) / "empty.xml"
            _write(xml_path, "")
            self.assertEqual(2, check_coverage.main(["check_coverage.py", str(xml_path), "0.95"]))

    def test_given_wrong_root_then_returns_2(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            xml_path = Path(tmp) / "wrong.xml"
            _write(xml_path, "<root></root>")
            self.assertEqual(2, check_coverage.main(["check_coverage.py", str(xml_path), "0.95"]))

    def test_given_missing_counts_and_line_rate_then_returns_2(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            xml_path = Path(tmp) / "missing.xml"
            _write(
                xml_path,
                """<?xml version="1.0"?><coverage><packages><package name="p" /></packages></coverage>""",
            )
            self.assertEqual(2, check_coverage.main(["check_coverage.py", str(xml_path), "0.95"]))

    def test_given_no_packages_then_returns_1(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            xml_path = Path(tmp) / "nopackages.xml"
            _write(xml_path, """<?xml version="1.0"?><coverage lines-covered="1" lines-valid="1"></coverage>""")
            self.assertEqual(1, check_coverage.main(["check_coverage.py", str(xml_path), "0.95"]))

    def test_given_line_rate_then_returns_0(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            xml_path = Path(tmp) / "linerate.xml"
            _write(xml_path, _cobertura_xml_with_line_rate(0.96))
            self.assertEqual(0, check_coverage.main(["check_coverage.py", str(xml_path), "0.95"]))

    def test_given_invalid_lines_valid_then_returns_2(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            xml_path = Path(tmp) / "invalidcounts.xml"
            _write(
                xml_path,
                """<?xml version="1.0"?>
<coverage lines-covered="NaN" lines-valid="100">
  <packages><package name="p" /></packages>
</coverage>
""",
            )
            self.assertEqual(2, check_coverage.main(["check_coverage.py", str(xml_path), "0.95"]))

    def test_given_exact_threshold_then_returns_0(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            xml_path = Path(tmp) / "ok.xml"
            _write(xml_path, _cobertura_xml(95, 100))
            self.assertEqual(0, check_coverage.main(["check_coverage.py", str(xml_path), "0.95"]))

    def test_given_just_below_threshold_then_returns_1(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            xml_path = Path(tmp) / "low.xml"
            _write(xml_path, _cobertura_xml(94, 100))
            self.assertEqual(1, check_coverage.main(["check_coverage.py", str(xml_path), "0.95"]))


if __name__ == "__main__":
    unittest.main()

