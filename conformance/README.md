# Conformance Vectors

The executable form of the specs: inputs and the exact outputs an
implementation must produce. Language-neutral YAML, so the same vectors test
the Rust core, a future second implementation, or a plugin author's
assumptions. **Status: no software runs these yet** — they are what the
first implementation is written against.

| File | What it pins down | Spec |
|---|---|---|
| `validate.yaml` | Project files in → diagnostics out | [tag-model](../docs/specs/tag-model.md), [expressions](../docs/specs/expressions.md), [protocol-drivers](../docs/specs/protocol-drivers.md) |
| `expressions.yaml` | Expression + inputs → value and quality | [expressions](../docs/specs/expressions.md) |
| `decode.yaml` | Registers → values; raw → engineering units | [protocol-drivers](../docs/specs/protocol-drivers.md), [tag-model](../docs/specs/tag-model.md) |
| `publish.yaml` | Sample sequence → publish or not | [edge-agent](../docs/specs/edge-agent.md) |
| `commands.yaml` | Command + edge state → accepted or rejected | [command-path](../docs/specs/command-path.md) |

Rules:

- Every case cites the requirement it proves (`requirement: REQ-…`, from the
  [requirements index](../docs/requirements.md)). `scripts/requirements.py
  --check` fails on an identifier that does not exist.
- A diagnostic is matched on `severity`, `path`, and `requirement`. Message
  wording is free; codes are assigned by the implementation and then frozen.
- Numbers compare with absolute tolerance `1e-6` unless a case sets `tol`.
- A case with no `value` in its expectation asserts only the quality: a
  `BAD` result has no meaningful number.
- A spec change that alters behavior changes these files in the same pull
  request ([operations](../docs/specs/operations.md)).

Whole-site behavior over time is covered by simulation scenarios
(`examples/greenhouse/test/sim/scenarios/`), not here.
