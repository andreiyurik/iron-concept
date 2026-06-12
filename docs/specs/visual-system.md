---
status: draft
type: spec
depends-on: [tag-model, command-path, historian]
---

# Visual System

How IRON renders SCADA screens — from widget dashboards to full P&ID mnemonic
diagrams. Three layers, each serving a different user. LiveView handles ~95% of
runtime rendering; React exists only in the design-time editor.

A SCADA screen is not a web page: an operator must understand plant state in
two seconds by looking at the picture. The visual representation is a safety
tool.

## The three layers

```
Layer 3 — SVG Editor (React island)            DESIGN-TIME ONLY · Phase 3
  Drag-and-drop mnemonic builder for integrators
  Output: SVG files with data-iron-* attributes (the Layer 2 format)

Layer 2 — SVG Mimics (LiveView + ~80-line JS hook)   RUNTIME · Phase 2
  Static SVG drawn in Inkscape/Figma/AI + dynamic data overlay

Layer 1 — Widget Dashboards (pure LiveView)          RUNTIME · Phase 1
  Grid of ready-made components, YAML or mouse
  Covers ~70% of real-world use cases
```

A single screen can combine all three. Rationale for LiveView-first:
[decisions/0002-elixir-phoenix-liveview.md](../decisions/0002-elixir-phoenix-liveview.md).

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
- Each widget subscribes via PubSub to exactly its own tags — an open screen
  receives updates for visible tags only.

Trend charts are the one runtime JS dependency (uPlot, 45KB, zero deps),
fed by the historian's continuous aggregates with LOCF semantics
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

Mechanics: a `MimicLive` LiveView loads the SVG, extracts bound tags,
subscribes via PubSub, and pushes `{tag, value, quality}` events to an
~80-line JS hook that patches the DOM directly. Re-rendering the whole SVG
per update was rejected: 200 bound elements at 10Hz would mean 2,000 full
diffs/sec; the hook pushes 3 fields.

Workflow: draw in Inkscape/Figma (or generate with AI) → add bindings →
`iron validate --mimics` confirms every referenced tag exists → commit SVG to
Git. IRON ships a standard symbol library (pumps, valves, tanks, motors,
pipes, instruments) with binding placeholders.

## Layer 3 — Visual editor

A React island mounted via phx-hook, design-time only. An editor is a
fundamentally different application from a dashboard: client-authoritative
state, 60fps mouse interaction, imperative DOM. LiveView's server round-trip
per interaction is wrong for this; React is right. The operator never loads
the editor — its output is a Layer 2 SVG file committed to Git.

Editor scope (Phase 3): palette drag-and-drop, tag binding panel, pipe drawing
with auto-routing, snap/align/group, undo/redo, symbol editor, save → Git.

## Trade-offs accepted

- **SVG, not Canvas/WebGL.** SVG degrades above ~5,000 DOM elements. Operator
  screens run 100–500 elements; a whole-refinery view should be split into
  linked screens anyway. Canvas would sacrifice accessibility and free
  browser hit-testing for a scale IRON does not target.
- **No collaborative editing.** Mimic design is single-user in practice; Git
  handles merges.
- **uPlot as runtime JS.** The one exception to "no JS frameworks at runtime";
  server-rendered chart images would be worse on every axis.
- **No import of proprietary symbol formats.** SVG is the universal vector
  format; converters can be community plugins.

## Open questions

- ISA-5.1 symbol style vs. modern flat style designed for screens?
- Mobile: auto-simplified mimic views, or separate Layer 1 dashboards for
  phones? (Layer 1 adapts naturally; free-form SVG does not.)
- Offline viewing via service worker — useful for poor connectivity, adds
  complexity. The edge agent's local fallback page
  ([edge-agent.md](edge-agent.md)) covers the most critical case already.
