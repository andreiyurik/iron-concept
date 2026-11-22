---
status: draft
type: spec
depends-on: [tag-model, alarm-engine, cli, deployment]
---

# Configuration Lifecycle

How configuration gets from Git to a running plant, how a plant is tuned
while it runs, and how those changes get back into Git. Rationale:
[ADR 0013](../decisions/0013-bounded-runtime-adjustments.md).

## Three things, three names

| Name | What it is | Where it lives |
|---|---|---|
| **Committed config** | The spec files at a Git commit | Git |
| **Adjustments** | A small, audited set of value changes made on a running site | The site's journal |
| **Effective config** | Committed config with adjustments applied — what actually runs | Derived, on every target |

`effective = committed + adjustments`. With no adjustments the two are
identical, which is the normal state of a site.

## Config hash

- The **config hash** is a hash of the effective configuration in its
  expanded, canonical form (after device-type expansion, with adjustments
  applied). Two targets with the same hash run the same configuration.
- Every target records the commit it was deployed from and its current
  config hash. `iron version` and `iron diff` report both ([cli.md](cli.md)).
- The edge sends its config hash when it opens a session, and every command
  carries the hash the server validated it against
  ([transport.md](transport.md), [command-path.md](command-path.md)).

## Applying configuration

Whether triggered by `iron deploy` or by `iron dev` watching files, an apply
follows one sequence:

1. Validate the candidate with the same `iron-core` validator as
   `iron validate`. An invalid candidate is never applied.
2. Swap atomically. A target runs the old configuration or the new one,
   never a mixture.
3. On failure the old configuration keeps running and the failure is
   reported with standard diagnostics.

Normative rules:

- An apply MUST NOT lose buffered samples or alarm events. `REQ-CFG-001 integration`
- **Alarm state survives an apply.** An alarm whose definition did not
  change MUST keep its state (active, acknowledged, shelved). A deploy must not
  re-annunciate every standing alarm in the plant. `REQ-CFG-003 integration`
- An alarm whose definition changed is re-evaluated from `NORMAL` against
  the new definition; an alarm that was removed is closed with a recorded
  reason.
- In a multi-host site, edges and the server are applied one target at a
  time and may briefly disagree. The behavior during that window is the
  config-hash mismatch rule in [transport.md](transport.md): reading
  degrades, writing stops, and the window is visible.

## Adjustments

Plants are tuned while they run. An alarm limit that chatters at three in
the morning will be changed by whoever is on site, with or without a
workflow for it. IRON gives that change a narrow, audited path instead of
pretending it will not happen.

### What can be adjusted

An adjustment MUST be limited to exactly these alarm fields, and nothing
else: `REQ-CFG-004 integration`

| Field | Why it is adjustable |
|---|---|
| `limit` | Tuning a threshold to the real process |
| `deadband` (alarm hysteresis) | Stopping chatter at the limit |
| `on_delay` / `off_delay` | Filtering transients |

Everything else — sources, types, scaling, ranges, priorities, messages,
suppression conditions, tag deadbands, derived expressions, commands, device
types, dashboards — changes only through Git and a deploy. In particular,
no adjustment can create, remove, or re-target anything, and no adjustment
can touch `config/commands/`.

An alarm MAY opt out with `adjustable: false`; such an alarm changes only
through Git.

### Rules

- Making an adjustment requires the `engineer` role and a free-text reason.
- An adjustment is validated against the effective configuration with the
  same invariants as `iron validate` (limit within `range`, ordering of
  limits). An adjustment that would make the configuration invalid is
  refused.
- Every adjustment is written to the append-only journal before it takes
  effect: who, when, old value, new value, reason.
- An adjustment is not a command: it changes how IRON evaluates an alarm,
  not the state of a machine. It does not pass through the Command Service
  and cannot reach a PLC.
- An adjustment takes effect only when the component that evaluates the
  alarm has applied it. If the owning edge is offline, the adjustment is
  refused — there is no pending state to forget about.
- Adjustments change the config hash.
- Acknowledging and shelving are alarm *state*, not configuration, and are
  unaffected by this section ([alarm-engine.md](alarm-engine.md)).

## Editing configuration

People edit spec files in an editor. Programs edit them through `iron-core`
and in no other way
([ADR 0014](../decisions/0014-config-edits-as-core-operations.md)).

### The file format: a lossless subset of YAML

- Configuration files MUST be UTF-8 YAML restricted to: block mappings and
  sequences, single-line flow mappings and sequences for leaf values
  (`range: [0, 200]`), plain and quoted scalars, and comments. Anchors,
  aliases, merge keys, explicit tags, multi-document files, multi-line flow
  collections, and duplicate keys are not allowed; `iron validate` MUST
  reject them with a diagnostic. `REQ-CFG-005 integration`
- Reading a file and writing it back with no edit MUST reproduce it
  byte-for-byte. `REQ-CFG-006 integration`

### Edit operations

A closed set, implemented once in `iron-core`:

| Operation | Does |
|---|---|
| `set` | Change the value at an existing path, or add a field to an existing entry |
| `add` | Add a new entry (tag, alarm, instance, command, binding) to a file |
| `remove` | Remove an entry |

- An edit operation MUST leave every byte it did not change untouched —
  comments, ordering, indentation, quoting. `REQ-CFG-007 integration`
- An edit operation MUST validate the resulting project and MUST NOT write a
  result that fails validation; it returns the diagnostics instead. `REQ-CFG-008 integration`
- Edit operations write to a project working tree. They never commit, and
  they MUST NOT run against the configuration of a deployed target: a
  running site changes only by deploy or by adjustment. `REQ-CFG-009 integration`
- Every program that changes configuration — `iron diff --pull`,
  `iron generate`, `iron upgrade`, and any future form, binding tool, or
  agent tool — MUST do so through these operations. `REQ-CFG-010 integration`

In v1 the operations have three callers, all in the CLI. Forms in the web
UI and click-to-bind for mimics are *deferred*
([business/deferred.md](../business/deferred.md)); when they arrive they run
inside `iron dev` on a workstation and call the same operations.

## Drift, and the way back to Git

`iron diff` compares the committed config at Git HEAD with the effective
config on a target ([cli.md](cli.md)). There are exactly two kinds of
difference:

- **Adjustments** — expected, attributed, and reversible. Reported with
  author, time, and reason.
- **Anything else** — the target does not match any commit plus its
  journaled adjustments. This is unexplained drift and is reported as an
  error.

Adjustments return to Git with one command:

```bash
iron diff --target plant --pull
# writes each adjustment into the spec files in the working tree:
#   config/tags/zone_1.yaml  zone_1.air_temperature.alarms.low.limit  8 → 7
#     adjusted 2026-05-02 03:11 by arman@plant.kz — "chatters at dawn, heater lag"
# review the diff, commit, deploy
```

- `--pull` edits files; it never commits. Review is a human step.
- For a device-type instance, a pulled adjustment becomes an instance-level
  override ([device-types.md](device-types.md)), never an edit to the type.
- When a deploy delivers a commit whose value equals an adjustment, that
  adjustment is retired: the commit is now the source of that value.
- **`iron deploy` MUST refuse** to deploy onto a target holding adjustments
  that the commit being deployed does not contain, and MUST name them. The
  operator either pulls them first or passes `--discard-adjustments`, which
  is itself journaled. A tuning change is never lost silently, in either
  direction. `REQ-CFG-002 integration`

## Phasing

The effective-config model and the config hash are part of the first
implementation — they decide what "the running configuration" means.
The adjustment API and its UI arrive with alarm management in Phase 2
([roadmap](../business/roadmap.md)).
