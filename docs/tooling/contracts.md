# Contract Verification Tooling

Contracts (`requires`/`ensures`) provide lightweight refinement checks for shapes and effects.
Tooling must surface unverifiable contracts with actionable guidance.

## Responsibilities

- Emit contract obligations into `hints.json` with suggested fixes or proof strategies.
- Allow `chic explain` to highlight failing contracts and show relevant MIR values.
- Integrate with the schedule tuner so memory/latency budgets derived from contracts inform search
  space pruning.

## Planned CLI

```
chic contract verify --symbol MyModule::fn \
    --report reports/contracts/MyModule_fn.md
```

Produces a Markdown summary showing which contracts were proven at compile time, which rely on
runtime assertions (debug builds), and suggested follow-up actions.
