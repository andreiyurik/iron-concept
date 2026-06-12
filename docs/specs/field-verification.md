---
status: draft
type: spec
depends-on: [tag-model, command-path, cli]
---

# Field Verification (`iron field`)

The bridge between simulation and production — and the feature that exists in
no other SCADA platform.

When hardware is connected, every tag must be verified: the physical signal
reaches the system, the engineering conversion is correct, the wiring matches
the configuration. Traditionally this is done with Excel spreadsheets, paper
checklists, and a radio. In IRON it is a first-class product workflow with
results stored in Git.

## The two-window workflow

The commissioning engineer connects a laptop and opens two browser windows fed
by the same live PubSub stream:

```
Window 1: /dashboard/reactor_01          Window 2: /field
┌─────────────────────────────┐          ┌──────────────────────────────────────┐
│  Mnemonic with live values  │          │  Field Verification   [AI][DI][AO][DO]│
│  Temperature: 87.5°C ● GOOD │          │  Tag              Val     Qual  Done  │
│  Pump 01:     RUNNING ● GOOD│          │  reactor_01.temp  87.5°C   ✅    ✅   │
│                             │          │  pump_01.running  FALSE    ✅    ⬜   │
│  "Does the signal make      │          │  pump_02.running  FALSE    ⚠️    ❌   │
│   sense in process context?"│          │  "Have I verified every single tag?" │
└─────────────────────────────┘          └──────────────────────────────────────┘
```

```
Technician (radio):  "Closing DI-003 now."
Engineer (browser):  watches pump_01.running: FALSE → TRUE   ✅
                     clicks ✅ in /field, glances at the mimic — pump shows RUNNING
                     "Good, next."
```

The mimic confirms the signal makes sense in context. The field table confirms
every tag has been verified systematically. Neither alone is sufficient;
together they are complete.

## Verification protocols by signal type

The protocol for each tag is **derived from its spec** — signal type, range,
and scaling come from `config/tags/`, so the checklist writes itself.

**AI — analog input** (4-20mA):

```
TAG: reactor_01.temperature   Range: 4mA = 0°C, 20mA = 200°C
Step 1: Apply 4mA.   → Reading: 0.1°C    [tolerance ±1°C]  ✅
Step 2: Apply 20mA.  → Reading: 199.8°C                    ✅
Step 3: Apply 12mA.  → Reading: 100.2°C                    ✅
```

**DI — digital input:**

```
Step 1: Ensure DI-003 OPEN.   → Reading: FALSE  ✅
Step 2: Close DI-003.         → Reading: TRUE   ✅
```

**AO / DO — outputs.** These are WRITE operations and run through the full
command path with explicit authorization:

```
TAG: pump_01.start_cmd
⚠️  WRITE OPERATION — requires role: operator
Step 1: IRON will energize DO-005 for 2 seconds.
        Authorized by: aigerim@plant.kz
        Confirm pump starts at field.  → ✅
```

## Normative rules

- AI/DI verification performs no writes — safe to run at any time.
- AO/DO verification requires the `operator` role minimum; every pulse is an
  audited command with the standard lifecycle
  ([command-path.md](command-path.md)).
- Tags wired to protective functions require the `engineer` role and an
  explicit `--allow-safety` flag; IRON never exercises an E-stop circuit —
  safety loop proof tests follow the plant's own safety procedures.
- A verification session is resumable: progress persists, and the report
  distinguishes verified / failed / skipped / pending.

## Selective runs

```bash
iron field --target edge-01          # everything
iron field --tag reactor_01.temperature
iron field --type AI                 # all analog inputs
iron field --object reactor_01
iron field --dry-run                 # show the plan, no writes
```

## Commissioning report

Every session generates a Markdown report, committed to Git:

```markdown
# Field Verification Report
Date: 2026-03-24 14:32 · Target: edge-01
Engineer: Arman Seitkali · Authorized by: Aigerim Bekova (operator)

| Tag | Type | Step | Expected | Actual | Result |
|---|---|---|---|---|---|
| reactor_01.temperature | AI | 4mA  | 0.0°C   | 0.1°C   | ✅ |
| reactor_01.temperature | AI | 20mA | 200.0°C | 199.8°C | ✅ |
| pump_01.running        | DI | CLOSE| TRUE    | TRUE    | ✅ |
| pump_01.start_cmd      | DO | ON   | —       | confirmed | ✅ |

47 tags verified · 0 failures · 2 warnings
⚠️ reactor_01.flow — not verified (source unreachable)
⚠️ pump_02.status — skipped by engineer
```

Saved as `reports/field/2026-03-24-edge-01.md`. Author, timestamp, and results
are permanently in version history — the commissioning record survives the
laptop, the engineer, and the integrator.

## Why this matters

Signal checkout is days of every commissioning project, it gates acceptance
payment, and its record-keeping today is Excel + paper + trust. Making it a
repeatable, audited, Git-versioned product workflow is worth more to a working
integrator than any dashboard feature — and no incumbent offers it.
