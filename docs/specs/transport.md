---
status: draft
type: spec
depends-on: [tag-model, command-path, read-write-separation, config-lifecycle]
---

# Transport Contract

What the edge role and the server role say to each other, and what each may
assume about delivery. This contract is independent of the bus that carries
it. IRON has two *bindings* of it: an in-process channel (the default
single-binary deployment) and NATS JetStream
([ADR 0003](../decisions/0003-nats-jetstream.md)). Both MUST satisfy
everything on this page; application code is written against the contract,
never against a binding. `REQ-TRN-001 integration`

## Addresses

- A **tag address** is the tag's canonical dotted path
  (`zone_1.air_temperature`) — the same string in specs, the historian,
  dashboards, and expressions ([tag-model.md](tag-model.md)).
- The first segment of a path is its **area** (`zone_1`).
- A **site** is one IRON deployment; its id defaults to the project name.
- Every area is owned by exactly one edge. In the default deployment the one
  process owns every area. With several edges, each edge target lists the
  areas it owns in `config/deploy.yml`; `iron validate` MUST reject an area
  owned by two edges or by none. `REQ-TRN-002 integration`
- The reserved `_iron` area is the one exception: every edge publishes its
  own system tags under `_iron.<edge id>.…`
  ([protocol-drivers.md](protocol-drivers.md)).

## Message kinds

| Kind | Direction | Carries | Delivery |
|---|---|---|---|
| Sample | edge → server | tag, value, quality, timestamp | At-least-once, durable |
| Alarm event | edge → server | tag, alarm name, transition (ACTIVE / RTN), timestamp | At-least-once, durable |
| Gap marker | edge → server | areas, interval that was lost | At-least-once, durable |
| Command | server → edge | envelope defined in [command-path.md](command-path.md) | **At-most-once, never stored** |
| Command result | edge → server | command id, state, observed value, reason | At-least-once |
| Hello / welcome | edge ↔ server | identities, protocol version, config hash | Request–reply |
| Heartbeat | edge ↔ server | timestamps for clock-offset estimation | Request–reply |

## Guarantees

### Data (samples, alarm events, gap markers)

- The edge's local buffer is the source of durability
  ([edge-agent.md](edge-agent.md)). A message leaves the buffer only after
  the binding has acknowledged it.
- Delivery is at-least-once. Receivers MUST be idempotent: a sample's
  identity is `(tag, timestamp)`; an alarm event's identity is
  `(tag, alarm, transition, timestamp)`. `REQ-TRN-003 integration`
- Per tag, the edge sends in timestamp order. Across tags, and after a
  replay, the server MUST accept any interleaving and any lateness. `REQ-TRN-004 integration`
- **Replay must not starve live data.** While replaying a backlog the edge
  MUST keep sending current samples and alarm events at their normal
  latency; the backlog drains with whatever capacity remains. `REQ-TRN-005 integration`
- If the buffer overflowed, the edge sends a gap marker so the loss is
  recorded rather than invisible ([historian.md](historian.md)).

### Commands

- A command is delivered **at most once and is never stored by the
  transport**. If the owning edge has no live session at the moment of
  issue, the command is not delivered and its lifecycle ends in `EXPIRED`
  ([command-path.md](command-path.md)). There is no queue a command can wait
  in.
- The transport never retries a command. Re-issuing is an operator action
  and produces a new command id.
- Only the Command Service can send a command; only the owning edge can
  receive it ([read-write-separation.md](read-write-separation.md)).

### Session

- An edge opens a session with a hello carrying its edge id, the transport
  protocol version, and its **config hash**
  ([config-lifecycle.md](config-lifecycle.md)). The server answers with its
  own.
- A protocol version the server does not support MUST fail the session with
  a diagnostic, not degrade silently. `REQ-TRN-006 integration`
- A **config-hash mismatch does not stop data**: samples for tags both sides
  know are accepted, samples for unknown tags are dropped and counted, and
  the server raises a visible system alarm naming the edge. Commands to a
  mismatched edge are refused. Reading degrades; writing stops.
- Heartbeats are request–reply and carry timestamps from both ends; the edge
  maintains an estimate of its clock offset from the server. Command TTL is
  evaluated with that estimate ([command-path.md](command-path.md)).
- A session with no successful heartbeat for 3 intervals (default interval
  2s) is dead: the server marks the edge offline, and commands to it expire
  immediately.

## Bindings

### In-process (default deployment)

Edge and server roles run in one process and exchange the same message
types over bounded channels. Durability comes from the same local buffer;
there is one clock, so the offset is zero; the config hash is trivially
equal.

### NATS JetStream (plant mode)

| Message kind | Subject | Mechanism |
|---|---|---|
| Sample | `data.<site>.<tag path>` | JetStream stream |
| Alarm event | `alarm.<site>.<tag path>.<alarm>` | JetStream stream |
| Gap marker, hello, heartbeat | `edge.<site>.<edge id>.<kind>` | JetStream (gap) · core request–reply (hello, heartbeat) |
| Command | `cmd.<site>.<command path>` | **Core NATS only** — no stream may capture `cmd.>` |
| Command result | `result.<site>.<command path>` | JetStream stream |

- The five roots are disjoint; no subject belongs to two.
- Wildcards follow the plant hierarchy: `data.<site>.zone_1.>` is all of
  zone 1.
- Per-identity subject permissions are specified in
  [security.md](security.md).
- JetStream adds what the buffer alone does not: the server can restart and
  catch up from the stream, and plugins can subscribe without the edge
  knowing about them ([extensions.md](extensions.md)).

A direct edge-to-server binding with no broker is possible under this
contract and is *deferred* with its trigger in
[business/deferred.md](../business/deferred.md).

## Encoding and versioning

- Message types are defined once, in `iron-domain`. The v1 encoding is JSON
  with schemas generated from those types — chosen so a human can read a
  subject with standard tools. A binary encoding is adopted only if the
  scaling benchmark ([architecture.md](architecture.md)) shows encoding to
  be the bottleneck.
- The protocol version is a single integer in the hello. A server MUST
  support the current version and the previous one, so edges and servers
  can be upgraded one at a time. `REQ-TRN-007 integration`
