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
| **Supervisory automation** | Rules and schedules that issue commands ("open the vent above 30°C", "close the valve at 2am"), declared in `config/commands/` and run through the full Command Path with a service identity | Closed-loop logic belongs in a controller next to the process; an automation engine is a second product, and getting it wrong moves a machine | A real Level 1 site with no controller that would otherwise choose Home Assistant or Node-RED over IRON for this reason alone — and never as a side effect of alarms | [specs/command-path.md](../specs/command-path.md), [guides/hardware.md](../guides/hardware.md) |
| **Stateful expression functions** | Rate of change, moving average, totalizer, timers, previous value in the expression language | Need memory across evaluations and defined behavior across restarts and gaps; plugins cover them meanwhile | The same stateful calculation written as a plugin at two different sites | [specs/expressions.md](../specs/expressions.md), [ADR 0012](../decisions/0012-one-expression-language.md) |
| **Command permissives** (`permitted_when`) | A boolean expression on a command that must hold for the Command Service to accept it, shown to the operator as the reason a button is disabled | Real interlocks belong in the PLC; a server-side copy can disagree with it, and it would put an expression context on the WRITE path | Operators at a real site repeatedly issue commands the PLC then refuses, and want to see why before clicking | [specs/command-path.md](../specs/command-path.md) |
| **Direct edge-to-server binding** | A third transport binding: an edge connects straight to the server over one authenticated stream, no broker | Two bindings already double the transport test matrix; NATS is one small binary | A site with one or two remote edges for which operating a broker is the stated obstacle | [specs/transport.md](../specs/transport.md), [ADR 0003](../decisions/0003-nats-jetstream.md) |
| **Automatic server failover** | Two or more servers with automatic promotion: database quorum (three voters), a single active Command Service, alarm state and command journal that move with it | Needs its own spec; a standby switched over by a person covers sites that tolerate minutes of supervisory downtime, which is most of them | First site whose requirement is supervisory availability without a person in the loop | [specs/deployment.md](../specs/deployment.md) |
| **WASM modules** (edge + server) | Sandboxed, language-agnostic in-process modules: stateful calculations, alarm enrichment, notification channels | Two host interfaces to design and keep stable for zero users; expressions, plugins, and REST cover every known need today | First extension that neither an expression nor a plugin can do (latency-critical stateful calculation, or must run on the edge without a network) | [specs/extensions.md](../specs/extensions.md), [ADR 0008](../decisions/0008-extensions-beyond-elixir.md) |
| **`iron module` command** | Vendored install/list/remove of WASM modules | No WASM in v1 | Arrives with WASM modules | [specs/extensions.md](../specs/extensions.md) |
| **`iron contract diff`** | Semver + breaking-change detection across the extension contracts and REST/WS API | One contract in v1 (plugin tag/subject contract); a diff tool for one contract is a changelog | Second stable contract, or the first community plugin breaks on an upgrade | [specs/extensions.md](../specs/extensions.md) |
| **Split-role deployment** (`iron server --role ui` / `--role core`) | Same binary on two hosts: UI role in a DMZ with no command credentials; Command Service in the protected zone | Implied by existing decisions and cheap later; no v1 site has a DMZ | First plant with an IEC 62443-style DMZ or internet-reachable dashboards | [specs/security.md](../specs/security.md) (contour rule), this page (mode details below) |
| **Configuration forms in the web UI** | Edit tags, alarms, and commands in a form inside `iron dev`; the form calls the core edit operations and the result is a normal Git diff | v1 serves the text-comfortable integrator; the operations it would call exist first | A real engineer who will not edit YAML and has no integrator to do it — observed, not assumed | [specs/config-lifecycle.md](../specs/config-lifecycle.md), [ADR 0014](../decisions/0014-config-edits-as-core-operations.md) |
| **Click-to-bind for mimics** | In `iron dev`, click an SVG element and pick its tag or command; writes the `data-iron-*` attributes through a core edit operation. Not an editor — drawing stays in Inkscape/Figma | Hand-typing attributes is unpleasant but works; needs lossless SVG attribute edits specified first | The first integrator building mimics for a real site reports binding, not drawing, as the time sink | [specs/visual-system.md](../specs/visual-system.md) |
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
| **Spec-file JSON Schemas in the scaffold** | `iron new` ships JSON Schemas for tags/commands/dashboards (generated from the same Rust types as the WS protocol) + a `yaml-language-server` association line — autocomplete, inline validation, and hover docs in any editor with zero extension code | Schemas are generated from types that do not exist yet | The Phase 1 types stabilize — this is the first deferred item expected to return, possibly within v1 | [specs/agent-interface.md](../specs/agent-interface.md) |
| **`iron lsp`** | A thin LSP server over the same core as the CLI and MCP: validator diagnostics inline in YAML, hover = `iron explain`, go-to-definition and rename across tag references. Agents get MCP, humans get LSP — the third thin facade | `iron dev`'s browser overlay already closes the edit-see-fix loop for v1; the diagnostic format is already LSP-compatible, so waiting costs nothing | First integrator routinely editing specs outside `iron dev` | [specs/agent-interface.md](../specs/agent-interface.md) |
| **VS Code extension** (mimic preview, simulator panel) | What LSP cannot do: live SVG mimic preview with binding overlay, dashboard preview, a simulator control panel in the editor | Editor-specific UI for zero users; the LSP tier must exist and prove demand first | A real user living in VS Code asks for more than `iron lsp` gives | [specs/visual-system.md](../specs/visual-system.md) |
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
