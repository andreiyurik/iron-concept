---
status: draft
type: business
---

# Deferred, Not Dropped

Ideas that are part of IRON's design but **not part of the first
implementation**. Each one was argued for, some were specified in detail, and
all were moved here for the same reason: the first release must be small
enough to ship and to learn from. Rails was extracted from Basecamp, not
designed ahead of it; IRON's v1 is the greenhouse and the pump station, and
the framework is what survives contact with them.

Each entry names the **trigger** that brings it back. Nothing here returns
because it is elegant; it returns because someone needed it.

| Idea | What it is | Why not in v1 | Trigger to revisit | Where detailed |
|---|---|---|---|---|
| **WASM modules** (edge + server) | Sandboxed, language-agnostic in-process modules: derived tags, alarm enrichment, notification channels | Two host interfaces to design and keep stable for zero users; plugins + REST cover every known need today | First extension that a plugin cannot do (latency-critical derived tag, or must run on the edge without a network) | [specs/extensions.md](../specs/extensions.md), [ADR 0008](../decisions/0008-extensions-beyond-elixir.md) |
| **`iron module` command** | Vendored install/list/remove of WASM modules | No WASM in v1 | Arrives with WASM modules | [specs/extensions.md](../specs/extensions.md) |
| **`iron contract diff`** | Semver + breaking-change detection across the extension contracts and REST/WS API | One contract in v1 (plugin tag/subject contract); a diff tool for one contract is a changelog | Second stable contract, or the first community plugin breaks on an upgrade | [specs/extensions.md](../specs/extensions.md) |
| **Split-role deployment** (`iron server --role ui` / `--role core`) | Same binary on two hosts: UI role in a DMZ with no command credentials; Command Service in the protected zone | Implied by existing decisions and cheap later; no v1 site has a DMZ | First plant with an IEC 62443-style DMZ or internet-reachable dashboards | [specs/security.md](../specs/security.md) (contour rule), this page (mode details below) |
| **Visual mimic editor** (Layer 3) | Drag-and-drop SVG editor with tag-binding panel, design-time only | Inkscape/Figma + `data-iron-*` attributes already *are* an editor; building one before a paying integrator asks is the classic trap | An integrator who would pay for it, or three who ask | [specs/visual-system.md](../specs/visual-system.md) |
| **Browser system tests** (Playwright) | End-to-end "frame became a pixel" tests in CI | Slow and brittle; Vitest on widgets + the integration chain test + `iron test --sim` cover behavior | A class of UI regressions that the unit layer demonstrably misses | [specs/testing.md](../specs/testing.md) |
| **Context-pack size budget & splitting** | `iron explain --area` enforcing a token budget and splitting by hierarchy | v1 areas are small; the rule matters at refinery scale | First site where one area's pack exceeds what an agent can use | [specs/agent-interface.md](../specs/agent-interface.md) |
| **Published manifest JSON Schema** | Versioned schema for the derived plant manifest | Schema stabilizes by being used; publishing v0 invites dependence on it | First external tool (not IRON's own) reading the manifest | [specs/agent-interface.md](../specs/agent-interface.md) |
| **CLI ↔ MCP parity enforced by a CI test** | Registry-generated command table + tool list, asserted equal | v1 has three MCP tools; parity is checked by eye | Fourth MCP tool | [specs/agent-interface.md](../specs/agent-interface.md) |
| **AI-parity metric** (agent vs. new engineer) | Per-release measurement of "add a plant area" by an agent with MCP+skill vs. a human with the quickstart | Needs a stable quickstart and a stable MCP first | End of Phase 1 — the first measurement is the Phase 2 entry criterion | [specs/agent-interface.md](../specs/agent-interface.md) |
| **Machine-applicable `fix` in diagnostics** | A diagnostic carries a patch an agent can apply verbatim | `hint` text is enough for agents today; `fix` needs a patch format | Evidence that agents misapply hints in a pattern a patch would prevent | [specs/agent-interface.md](../specs/agent-interface.md) |
| **Runtime-loadable custom widgets** | A Svelte widget written outside IRON, loaded into the embedded UI without rebuilding the binary | The UI is compiled into the binary; loading foreign components at runtime needs a shared Svelte runtime or an `iron build` step with a Node toolchain — both real work. Today a custom widget is a PR to the built-in library, and SVG mimics cover "my own picture" | An integrator needs a widget the library lacks and cannot wait for a release | [specs/visual-system.md](../specs/visual-system.md) |
| **Supported-devices page** | A generated, searchable list of every PLC / sensor / protocol with a core or verified driver, with the tier shown — the discovery engine Home Assistant's integrations page is | Needs more than one driver to list | Third verified driver | [specs/extensions.md](../specs/extensions.md) |
| **Community plugin registry** | An index of community plugins and templates (vendoring stays the install mechanism) | Vendoring + Git is enough until there is something to index | Ten published plugins or templates | [specs/extensions.md](../specs/extensions.md) |
| **Immutable firmware images for edge** | Yocto/Buildroot + A/B OTA (RAUC/Mender-style) as an alternative to Kamal on edge devices | Kamal + a static binary covers Phase 1–2 hardware | First harsh-environment deployment where container updates are the wrong tool | [specs/deployment.md](../specs/deployment.md), [guides/hardware.md](../guides/hardware.md) |
| **PLC runtime** | Open Rust IEC 61131-3 runtime on `plc-lang/rusty` | Long-term exploration, gated separately | Per [specs/plc-runtime.md](../specs/plc-runtime.md) and [ADR 0007](../decisions/0007-codesys-today-iron-plc-later.md) | [specs/plc-runtime.md](../specs/plc-runtime.md) |

## Split-role deployment — the details, parked

Kept here verbatim so the design is not lost; the rule it enforces lives in
[security.md](../specs/security.md).

```
iron server --role ui      DMZ / office VLAN    dashboards · REST read · WebSocket · MCP (read-only)
                           NATS identity: subscribe data.> only. No cmd.> credentials exist on this host.
iron server --role core    protected IT VLAN    Command Service · historian writer · alarm management
                           NATS identity: the only cmd.> publisher. No inbound from the DMZ except NATS.
```

| | single process (default) | split roles |
|---|---|---|
| Command Service credentials | on the server host | **only** on the `core` host |
| UI host compromised ⇒ can command a machine? | prevented by code + broker authz | prevented by code + broker authz **+ the credentials are not there** |
| Config, specs, `iron validate` output | identical | identical |
| Extra operational surface | — | one more host, one more Kamal target |

Rules when implemented: a `--role ui` process MUST NOT be issued credentials
with any `cmd.>` rights and MUST refuse to start if it finds them; command
submissions from the UI role reach the Command Service only through the
broker on an authenticated request subject, and the `core` role applies the
full WRITE path ([command-path.md](../specs/command-path.md)); `iron deploy`
treats the roles as two Kamal targets, and a `ui`-only deploy MUST NOT touch
the `core` host.

## How an idea leaves this page

1. Its trigger fires — a real site, a real contributor, a real failure.
2. The spec section it points to is promoted from "deferred" to normative
   (status in frontmatter, no new ADR unless the decision itself changes).
3. The row is removed here and the roadmap phase that absorbs it is updated.

The opposite direction is allowed too: a v1 feature that no site uses by the
end of Phase 2 comes here, with its own trigger.
