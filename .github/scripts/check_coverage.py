#!/usr/bin/env python3

from __future__ import annotations

import sys
import xml.etree.ElementTree as ET


EPS = 1e-12


def main(argv: list[str] | None = None) -> int:
    argv = argv if argv is not None else sys.argv
    if len(argv) != 3:
        print("Usage: check_coverage.py <cobertura.xml> <threshold>", file=sys.stderr)
        return 2

    cobertura_path = argv[1]
    try:
        threshold = float(argv[2])
    except ValueError:
        print(f"Invalid threshold value: {argv[2]!r}", file=sys.stderr)
        return 2

    try:
        tree = ET.parse(cobertura_path)
    except FileNotFoundError:
        print(f"Coverage file not found: {cobertura_path}", file=sys.stderr)
        return 2
    except ET.ParseError as e:
        print(f"Invalid coverage XML: {e}", file=sys.stderr)
        return 2
    root = tree.getroot()
    if root.tag != "coverage":
        print(f"Unexpected root element: {root.tag}", file=sys.stderr)
        return 2

    packages = root.findall("./packages/package")
    if len(packages) == 0:
        print(
            "No coverage packages found in report. Failing build to avoid false-positive coverage.",
            file=sys.stderr,
        )
        return 1

    # Prefer explicit counts when present.
    covered_raw = root.attrib.get("lines-covered")
    valid_raw = root.attrib.get("lines-valid")
    if covered_raw is not None and valid_raw is not None:
        try:
            lines_covered = int(float(covered_raw))
            lines_valid = int(float(valid_raw))
        except ValueError:
            print(
                f"Invalid lines-covered/lines-valid values: {covered_raw!r}/{valid_raw!r}",
                file=sys.stderr,
            )
            return 2
        if lines_valid > 0:
            line_rate = lines_covered / lines_valid
        else:
            line_rate = 0.0
    else:
        line_rate_raw = root.attrib.get("line-rate")
        if line_rate_raw is None:
            print("Missing coverage line-rate attribute.", file=sys.stderr)
            return 2
        try:
            line_rate = float(line_rate_raw)
        except ValueError:
            print(f"Invalid line-rate value: {line_rate_raw!r}", file=sys.stderr)
            return 2

    percent = line_rate * 100.0
    threshold_percent = threshold * 100.0
    print(f"Line coverage: {percent:.2f}% (threshold: {threshold_percent:.2f}%)")

    if line_rate < threshold - EPS:
        print("Coverage threshold not met.", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

