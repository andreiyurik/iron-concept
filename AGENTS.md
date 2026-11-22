# Working on iron-concept

This repository is a **documentation-only engineering blueprint** for IRON —
an open-source SCADA platform. There is no code yet, by design. If you are an
agent (or a human) editing it, these are the rules everything else follows.

## The two hard rules

1. **English only.** No Russian (or any other language) anywhere in the repo —
   docs, diagrams, comments. Discussions and issues may be bilingual; files
   may not.
2. **Honest by policy.** Every performance number is either a cited external
   benchmark or explicitly labeled a **target**. IRON has no measurements of
   itself. Committed scope is supervisory SCADA only; the PLC runtime is
   exploration and must always be visibly gated as such. If a sentence reads
   as marketing where engineering should be, it is a bug.

## Structure and conventions

- **`docs/specs/` is normative** (MUST/SHOULD, testable). `docs/vision/` and
  `docs/business/` are narrative. `docs/decisions/` holds ADRs — one decision
  per file, with trade-offs; the template is in `docs/decisions/README.md`.
- **One fact lives in exactly one place** — link, don't copy.
- Every file in `docs/{specs,decisions,vision,business,guides}/` carries
  frontmatter: `status:` (`draft`/`accepted`/`superseded`/`exploration`) and
  `type:`.
- **`docs/business/deferred.md` is the parking lot**: ideas kept out of v1,
  each with an explicit return trigger. Never re-add a deferred item to the
  specs without its trigger firing; never delete an idea — defer it.
- **The complexity budget** (`docs/specs/cli.md`): a project containing only
  `config/tags/` is complete; no feature may introduce a new required file.
- `examples/greenhouse/` must always conform to the current specs — if a spec
  change breaks it, fix both together.

## Naming (do not mix these up)

| Name | Meaning |
|---|---|
| `iron-edge` | the edge agent (Rust, OT side) |
| `iron-server` | the server (Rust: tag engine, historian, Command Service, UI hosting) |
| `iron-domain` | pure shared logic (tags, quality, deadband, ISA-18.2), no I/O |
| `iron-core` | the operations library (manifest, validator, explain, generate) that CLI, MCP, and a future LSP wrap |
| plugin | an out-of-process extension in any language, own credentials |
| WASM module | in-process sandboxed extension — deferred, future `iron module` |

## Architecture invariants (never weaken in any edit)

- READ and WRITE are separate paths; nothing but the Command Service writes;
  alarms never act; an LLM is never on the WRITE path.
- No LLM runs inside IRON — the product exposes an agent interface
  (`docs/specs/agent-interface.md`), it never embeds a model.
- Config is text in Git; the manifest is derived, never hand-written. The
  only sanctioned runtime change is a bounded, journaled adjustment
  (`docs/specs/config-lifecycle.md`) — never widen that set in passing.
- One expression language (`docs/specs/expressions.md`). No spec may
  introduce a second mini-language, and no expression context may exist on
  the WRITE path.
- Edge and server talk through one contract (`docs/specs/transport.md`);
  specs describe the contract, never a broker's behavior. Commands are
  at-most-once and never stored.
- No implicit scaling: a value is converted only by an explicit `scaling`.
- Programs change configuration only through the `iron-core` edit
  operations, losslessly; config files stay inside the YAML subset
  (`docs/specs/config-lifecycle.md`). No anchors or merge keys in examples.

## Requirements and conformance

- A normative statement uses the word MUST, and every MUST carries an
  identifier and a test level inline (`REQ-CMD-003 integration`). Assign
  them with `python3 scripts/requirements.py`; never renumber or reuse one.
  `docs/requirements.md` is generated — do not edit it.
- `conformance/` holds the test vectors the implementation is written
  against. A spec change that alters behavior changes the vectors in the
  same commit.
- When code exists: a change that implements a requirement adds a test
  naming its identifier; third-party APIs are checked against the pinned
  version's documentation, not recalled (`docs/specs/operations.md`).

## Before committing

```bash
python3 scripts/requirements.py   # assign ids to new MUSTs, regenerate the index
python3 scripts/lint-docs.py      # links · English-only · frontmatter · honesty · requirements
```

Diagrams are hand-authored SVG in `docs/assets/`; preview with
`convert -background "#0d1117" -density 110 <file>.svg out.png` (CSS
gradients don't render in ImageMagick previews — check in a browser).
