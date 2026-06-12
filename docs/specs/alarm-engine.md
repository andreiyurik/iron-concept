---
status: draft
type: spec
depends-on: [tag-model, edge-agent, command-path]
---

# Alarm Engine

Alarms are how the system talks to humans about abnormal conditions. IRON's
alarm design follows the vocabulary and lifecycle of **ISA-18.2** (Management
of Alarm Systems for the Process Industries) — the standard that exists because
badly designed alarm systems have contributed to real industrial accidents.

## The prime rule: alarms notify, they never act

An IRON alarm MUST NOT trigger a command. No `action:` field exists in alarm
configuration, deliberately.

Why this is non-negotiable:

1. **Architecture.** The alarm engine is a READ-path consumer. An alarm that
   sends a command entangles READ and WRITE — the exact monolith mistake IRON
   exists to refuse ([read-write-separation.md](read-write-separation.md)).
2. **Physics.** An automatic protective reaction (trip, interlock, emergency
   shutdown) must not have a network, a broker, and two processes between cause
   and effect. That logic belongs in the PLC scan cycle — and for
   safety-rated functions, in a dedicated safety PLC. IRON *observes* interlock
   and trip states as tags; it is never the mechanism that enforces them.

If you find yourself wanting `alarm → command`, the correct designs are:

- **Protective action** → implement in the PLC / safety system; expose its
  status as a tag so IRON can display and historize it.
- **Operator-mediated response** → the alarm notifies; a human sends the
  command through the full WRITE path with RBAC and audit.

## Where alarms are evaluated

| Level | Component | Responsibility |
|---|---|---|
| Detection | Edge agent | Evaluate limits on every scan, before deadband; publish ACTIVE/RTN transitions; keep working during network loss ([edge-agent.md](edge-agent.md)) |
| Management | iron-web | State machine, acknowledgment, shelving, suppression, escalation, notification routing, alarm history |

Detection lives at the edge because the edge is the component guaranteed to be
alive next to the process. Management lives at the server because it needs
humans, history, and routing.

## Alarm state machine (ISA-18.2)

```
            limit crossed                 operator ack
  NORMAL ───────────────► UNACK_ACTIVE ───────────────► ACK_ACTIVE
     ▲                        │                              │
     │                        │ condition clears             │ condition clears
     │ operator ack           ▼                              ▼
     └─────────────── UNACK_RTN  ◄───────────────────────  NORMAL
                      (returned to normal, not yet acknowledged)
```

- Every transition is recorded in the historian with timestamp and (for ack)
  the operator identity.
- Acknowledgment requires the `operator` role minimum and is itself an audited
  action (but not a command — it changes alarm state, not machine state).

## Alarm definition

```yaml
# in config/tags/reactor_01.yaml
alarms:
  high:
    limit: 180
    priority: 2
    message: "High reactor temperature"
    deadband: 1.0          # alarm hysteresis: clears at 179.0, not 179.99
    on_delay: 3s           # condition must hold 3s before ACTIVE (anti-chatter)
    # Optional ISA-18.2 rationalization fields:
    cause: "Cooling water flow loss or exothermic excursion"
    consequence: "Product degradation above 185°C"
    operator_action: "Verify cooling pump P-101, reduce feed rate"
```

| Field | Required | Meaning |
|---|---|---|
| `limit` | yes | Threshold in engineering units (within tag `range`) |
| `priority` | yes | 1 (critical) – 4 (low); drives sort order, color, notification policy |
| `message` | yes | What the operator sees |
| `deadband` | no | Hysteresis for clearing — prevents chatter at the limit |
| `on_delay` / `off_delay` | no | Time qualification before transition |
| `cause` / `consequence` / `operator_action` | no | Rationalization; displayed in the alarm detail view |

`iron validate` enforces: limits within range, ordering
(`low_low < low < high < high_high`), priorities in 1–4.

## Alarm flood control

The most common failure of real alarm systems is not a missed alarm — it is
2,000 alarms in one minute after a section trip, burying the one that matters.

- **Shelving.** An operator may shelve a nuisance alarm for a bounded time
  (e.g. 8h max). Shelving is audited, visible in a "shelved" list, and expires
  automatically. Shelving MUST NOT be available for priority-1 alarms.
- **Suppression by design.** An alarm can declare a suppressing condition:
  `suppressed_when: pump_01.running == false` — a "low flow" alarm is
  meaningless when the pump is intentionally off. Suppressed alarms are
  evaluated but not annunciated; they appear in a "suppressed" list.
- **Flood detection.** If annunciation rate for an area exceeds a threshold
  (default: 10 alarms / 10 min / operator station, per ISA-18.2 guidance), the
  UI collapses to a priority-sorted summary view and the event is itself logged
  — flood episodes are reviewable afterwards.
- **First-out.** Alarm groups MAY declare first-out behavior: in a trip
  cascade, the first alarm in the group is flagged as the probable initiator.

## Escalation and notification

```yaml
# config/alarms/escalation.yaml
policies:
  critical:                       # applies to priority 1
    - notify: [shift_operator]    # immediately: UI + Telegram
    - after: 5m                   # unacknowledged after 5 minutes:
      notify: [shift_supervisor]
    - after: 15m
      notify: [plant_manager]
```

Notification channels are pluggable (Telegram first, then email/SMS/webhook).
Channel failures are logged and retried; a notification failure MUST be visible
in the UI ("escalation delivery failed"), never silent.

## Metrics

The alarm engine tracks and displays its own health, because an unhealthy alarm
system is a process hazard: annunciation rate per operator, % time in flood,
top-10 most frequent alarms (chattering candidates), shelved count and ages,
stale alarms (active > 24h). These map to the ISA-18.2 performance metrics and
are first-class dashboard widgets, not an afterthought.
