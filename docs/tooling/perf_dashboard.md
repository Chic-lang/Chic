# Static Performance Dashboard

The static dashboard renders `perf.json` and related artifacts as a self-contained HTML report. It
is generated via `chic perf report --dashboard` and requires no external services.

## Features

- Interactive tables for hotspots, budgets, and RNG usage.
- Timeline visualisations comparing baseline vs current runs.
- Direct links back to MIR instructions (`mir.json`) and source files.

## Output

- The dashboard emits `reports/perf_dashboard/index.html` plus supporting assets. All resources are
  embedded or shipped locally so the report can be shared offline.

## Implementation Notes

- The renderer is written in Rust using `askama` templates and minimal inline JS. Visuals rely on
  lightweight SVG charts to keep the file size small.
- All calculations are performed ahead of time; the HTML is static beyond simple toggles.
