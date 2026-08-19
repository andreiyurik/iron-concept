---
status: draft
type: spec
depends-on: [tag-model, command-path, historian]
---

# Visual System

How IRON renders SCADA screens — from widget dashboards to full P&ID mnemonic
diagrams. Three layers, each serving a different user. All three are Svelte 5
components served from the `iron` binary
([ADR 0010](../decisions/0010-svelte-for-the-ui.md)); the editor is
design-time only.

A SCADA screen is not a web page: an operator must understand plant state in
two seconds by looking at the picture. The visual representation is a safety
tool.

## The three layers

```
Layer 3 — SVG Editor (Svelte, design-time)          DEFERRED · Inkscape/Figma today
  Drag-and-drop mnemonic builder for integrators
  Output: SVG files with data-iron-* attributes (the Layer 2 format)

Layer 2 — SVG Mimics (one Svelte Mimic component)    RUNTIME · Phase 2
  Static SVG drawn in Inkscape/Figma/AI + reactive data binding

Layer 1 — Widget Dashboards (Svelte components)      RUNTIME · Phase 1
  Grid of ready-made components, YAML or mouse
  Covers ~70% of real-world use cases
```

A single screen can combine all three. Rationale for one Svelte stack:
[ADR 0010](../decisions/0010-svelte-for-the-ui.md); why the server is not
rendering the UI any more: [ADR 0009](../decisions/0009-rust-for-the-server.md).

## Layer 1 — Widget dashboards

```yaml
# config/dashboards/reactor_01.yaml
title: "Reactor 01 — Overview"
layout: grid
columns: 4

widgets:
  - type: gauge
    tag: reactor_01.temperature
    position: [0, 0]
    config:
      min: 0
      max: 200
      zones: { green: [0, 150], yellow: [150, 180], red: [180, 200] }

  - type: trend
    tags: [reactor_01.temperature, reactor_01.pressure]
    position: [1, 0]
    size: [2, 2]
    config: { timerange: 1h }

  - type: command_button          # WRITE path — see command-path.md
    command: reactor_01.pump_start
    position: [3, 0]

  - type: alarm_panel
    filter: "reactor_01.*"
    position: [0, 2]
    size: [4, 1]
```

Widget library:

```
Display (READ):  gauge · numeric · status · trend · sparkline · bar
                 tank_level · alarm_panel · table
Control (WRITE): command_button · setpoint · selector · slider
                 (all reference config/commands/ entries, never tags)
Layout:          group · mimic_embed · nav_link
```

Normative:

- Display widgets MUST render quality: a `BAD` value is visually distinct,
  never displayed as a plausible number.
- Control widgets MUST reference a command definition; binding a control
  widget to a tag is a validation error.
- Each widget subscribes over the typed WebSocket to exactly its own tags —
  an open screen receives updates for visible tags only.
- A widget is a Svelte 5 component against one contract: it declares the
  tags (display) or command (control) it binds, and receives
  `{value, quality, timestamp}` as reactive state. Custom widgets implement
  the same contract; `iron validate` rejects a widget that binds a control to
  a tag or a display to a command. **In v1 custom widgets reach users by
  being contributed to the built-in library** — the UI is embedded in the
  binary, and a runtime loading mechanism for widgets is *deferred* with its
  trigger in [business/deferred.md](../business/deferred.md). SVG mimics
  cover the "my own picture" need without any code.

Trend charts are the one non-Svelte runtime dependency (uPlot, 45KB, zero
deps), fed by the historian's continuous aggregates with LOCF semantics
([historian.md](historian.md)).

## Layer 2 — SVG mimics

Any SVG element binds to a tag with `data-iron-*` attributes:

```svg
<text data-iron-tag="reactor_01.temperature" data-iron-format="%.1f °C">--.-</text>

<circle data-iron-tag="pump_01.running"
        data-iron-class-true="fill-green-500"
        data-iron-class-false="fill-gray-400" cx="150" cy="300" r="25"/>

<rect data-iron-tag="tank_01.level"
      data-iron-fill="vertical" data-iron-range="[0, 100]"
      x="500" y="100" width="100" height="300"/>

<!-- WRITE: references a command, runs the full command path -->
<rect data-iron-command="pump_01.start_cmd" class="command-button"/>
```

### Binding contract

| Attribute | Purpose |
|---|---|
| `data-iron-tag` | Bind element to a tag |
| `data-iron-format` | Printf-style text format |
| `data-iron-class-true/false` | CSS class toggled by boolean |
| `data-iron-animate-true/false` | CSS animation toggled by boolean |
| `data-iron-fill` + `data-iron-range` | Proportional fill (tank level) |
| `data-iron-color-map` | Value→color gradient |
| `data-iron-visible-true/false` | Show/hide |
| `data-iron-quality` | Quality badge/border rendering |
| `data-iron-command` (+`-value`) | Click sends a command via the WRITE path |

Mechanics: one `Mimic` Svelte component loads the SVG, extracts bound
elements, subscribes to exactly those tags over the WebSocket, and patches
each bound attribute reactively when its tag changes. Re-rendering the whole
SVG per update is rejected by design: 200 bound elements at 10Hz would mean
2,000 full re-renders/sec; a bound element updates three attributes.

Workflow: draw in Inkscape/Figma (or generate with AI) → add bindings →
`iron validate --mimics` confirms every referenced tag exists → commit SVG to
Git. IRON ships a standard symbol library (pumps, valves, tanks, motors,
pipes, instruments) with binding placeholders.

## Layer 3 — Visual editor (deferred)

Inkscape, Figma, or an AI drawing an SVG with `data-iron-*` attributes already
*is* the editor: the file format is the contract, and `iron validate --mimics`
checks it. A built-in drag-and-drop editor — palette, tag-binding panel, pipe
auto-routing, snap/align, undo/redo, save → Git — is designed as a Svelte
application served only to design-time roles, never touching live plant
state, and is *deferred* until an integrator asks for it with money on the
table ([business/deferred.md](../business/deferred.md)).

## Trade-offs accepted

- **SVG, not Canvas/WebGL.** SVG degrades above ~5,000 DOM elements. Operator
  screens run 100–500 elements; a whole-refinery view should be split into
  linked screens anyway. Canvas would sacrifice accessibility and free
  browser hit-testing for a scale IRON does not target.
- **No collaborative editing.** Mimic design is single-user in practice; Git
  handles merges.
- **uPlot for trends.** The one non-Svelte runtime dependency; writing a
  high-frequency time-series chart in Svelte would be worse on every axis.
- **No import of proprietary symbol formats.** SVG is the universal vector
  format; converters can be community tools.

## Open questions

- ISA-5.1 symbol style vs. modern flat style designed for screens?
- Mobile: auto-simplified mimic views, or separate Layer 1 dashboards for
  phones? (Layer 1 adapts naturally; free-form SVG does not.)
- Offline viewing via service worker — useful for poor connectivity, adds
  complexity. The edge agent's local fallback page
  ([edge-agent.md](edge-agent.md)) covers the most critical case already.
