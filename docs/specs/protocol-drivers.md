---
status: draft
type: spec
depends-on: [tag-model, edge-agent, read-write-separation]
---

# Protocol Drivers

How the edge agent talks to field devices: connection parameters, protocol
addressing, how polls are grouped, and what happens when a device misbehaves.

> **v1 scope.** Modbus TCP only ([roadmap](../business/roadmap.md)). The
> driver contract, polling, and supervision rules below apply to every
> driver; addressing is specified for Modbus alone. Addressing sections for
> Modbus RTU, OPC-UA, and S7 are added when each driver is implemented, not
> ahead of it.

## The driver contract

A driver turns one *connection* and a set of *points* into samples.

- A driver exposes two separate capabilities: a **read handle** used by the
  polling engine and a **write handle** used only by the command executor.
  The polling engine never holds a write handle
  ([read-write-separation.md](read-write-separation.md)).
- A driver returns raw values and a per-point outcome (value, protocol
  exception, timeout). Conversion to engineering units, quality assignment,
  and deadband are `iron-domain` logic applied by the agent, not by the
  driver — so every driver gets identical semantics.
- The simulator is a driver (`sim://`) behind the same contract.

## Connections

A source URI names a host. The host is either an alias defined in
`config/connections.yaml` or a literal hostname or IP address.

```yaml
# config/connections.yaml — optional
plc-01:
  host: 192.168.10.20
  port: 502            # default 502
  unit: 1              # Modbus unit id, default 1
  order: ABCD          # byte order of 32- and 64-bit values, default ABCD
  timeout: 1s          # per request, default 1s
  retries: 1           # per request before the read counts as failed, default 1
  max_block: 125       # registers per read request, default 125
  max_gap: 0           # unrequested registers a block may read across, default 0
  concurrency: 1       # simultaneous requests on this connection, default 1
```

- `connections.yaml` is optional. A source with a literal host uses the
  defaults above. A project containing only `config/tags/` stays complete
  ([cli.md](cli.md), the complexity budget).
- An alias makes an address change a one-line diff and is the only place
  device-wide protocol parameters live.
- `iron validate` MUST reject an alias that is used but not defined when a
  `connections.yaml` exists and the name is not a syntactically valid
  hostname or IP address. `REQ-DRV-001 integration`

## Modbus addressing

```
modbus://<host>/<table>/<address>[.<bit>][?order=<order>]
```

| Part | Values |
|---|---|
| `table` | `coil` (read FC01, write FC05) · `discrete` (FC02) · `holding` (read FC03, write FC06/FC16) · `input` (FC04) |
| `address` | **Zero-based protocol address**, decimal or `0x` hex |
| `.bit` | Bit 0–15 of a 16-bit register, for `bool` tags packed into a status word |
| `?order=` | Per-tag override of the connection's byte order |

Normative rules:

- Addresses are zero-based protocol addresses — what goes on the wire.
  Datasheet notations such as `40001` or `4x0001` are **not** accepted in a
  source URI; `iron validate` MUST reject them with a hint giving the converted
  address. `iron generate from-io-list` performs the conversion on import
  ([device-types.md](device-types.md)). `REQ-DRV-009 integration`
- The tag `type` decides how many registers are read: `int16`/`uint16` one,
  `int32`/`uint32`/`float32` two, `float64` four. A `bool` reads one coil or
  discrete input, or one bit of a register when `.bit` is given.
- Decoding MUST follow `order`, which names the position of the value's bytes on the wire: `ABCD`
  (big-endian, the Modbus convention), `CDAB` (word-swapped), `BADC`
  (byte-swapped), `DCBA` (little-endian). The same four letters apply to
  64-bit values pairwise. A wrong `order` is the most common cause of a
  plausible-looking wrong number; [field verification](field-verification.md)
  is what catches it. `REQ-DRV-010 integration`
- A multi-register value MUST be read in a single request. It is never
  assembled from two requests. `REQ-DRV-002 integration`

## Polling

- Points on the same connection, table, and scan rate form a **scan group**.
- Within a scan group the driver MUST coalesce adjacent addresses into
  blocks of at most `max_block` registers, reading across at most `max_gap`
  unrequested registers. The default `max_gap` is 0 because some devices
  answer an exception for undefined registers. `REQ-DRV-003 integration`
- Every value decoded from one response MUST carry the same acquisition
  timestamp. `REQ-DRV-011 integration`
- A protocol exception for a block marks every point in that block `BAD`;
  the agent MUST then retry the affected points individually on the next
  scan so one bad address does not hide its neighbours, and report the
  offending address as a diagnostic. `REQ-DRV-004 integration`
- **Scan overrun.** If a scan group cannot complete within its scan rate,
  the agent MUST NOT queue scans. It skips to the next due time and counts
  the overrun. Overruns are visible, never silent. `REQ-DRV-005 integration`

## Supervision

Each connection runs as an independent supervised task.

- A slow, hung, or unreachable device MUST NOT delay polling of any other
  connection. `REQ-DRV-006 integration`
- Every request has a timeout. A connection that fails is closed and
  reopened with exponential backoff and jitter, capped at 30s by default.
- While a connection is down, its points follow the quality transitions in
  [edge-agent.md](edge-agent.md): stale → `UNCERTAIN`, then `BAD`.
- A panic inside a driver task MUST be contained to that connection: the
  task is restarted with the same backoff, and the agent keeps running. `REQ-DRV-007 integration`
- Writes share the connection but not the queue: a command write MUST NOT
  wait behind a full scan cycle. The command executor's request is sent
  ahead of pending reads. `REQ-DRV-008 integration`

### System tags

The agent publishes its own health as ordinary tags in the reserved `_iron`
area, so it is visible on dashboards and kept in history like plant data:

| Tag | Type | Meaning |
|---|---|---|
| `_iron.<edge id>.connections.<alias>.online` | bool | The connection is established and answering |
| `_iron.<edge id>.connections.<alias>.overruns` | uint32 | Scan overruns since agent start |

In v1 system tags are display and history only.

## Testing a driver

Core drivers run against a protocol simulator in `cargo test`; community
drivers run through `iron test --driver` ([extensions.md](extensions.md)).
Either way the conformance set includes: all four byte orders for every
multi-register type, block coalescing at the `max_block` boundary, an
exception inside a block, a timeout, a disconnect during a request, and a
scan overrun.
