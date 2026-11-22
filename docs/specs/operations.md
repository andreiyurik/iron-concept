---
status: draft
type: spec
depends-on: [deployment, config-lifecycle, testing]
---

# Operations and Engineering Gates

What it takes to run IRON for years and to trust code that is largely
machine-written: seeing inside the process, changing the configuration
format without stranding sites, getting a site back after a disk dies, and
the gates every change passes before it ships.

## Observability

- Every role MUST write structured logs (one JSON object per line: time,
  level, component, message, fields) to stdout. Secrets and command values
  beyond the audit journal's own content MUST NOT appear in logs. `REQ-OPS-001 ci`
- Every role MUST serve `GET /healthz` (the process is alive) and
  `GET /readyz` (it can do its job: configuration applied, storage writable,
  transport session up). Deploy health gates use `/readyz`. `REQ-OPS-002 ci`
- Every role MUST serve `GET /metrics` in Prometheus text format, bound to
  localhost unless configured otherwise. The minimum set: samples received
  and published, buffer depth and oldest buffered age, scan overruns,
  transport session state, command counts by final state, storage write
  latency, connected UI sessions. `REQ-OPS-003 ci`
- What matters to an operator is also a system tag
  ([protocol-drivers.md](protocol-drivers.md)); `/metrics` is for whoever
  maintains the installation. Distributed tracing is out of scope for v1.

## Configuration format version

- The configuration format has one integer version. A project states it in
  the optional file `config/iron.yaml` (`format: 1`); an absent file means
  version 1, so the complexity budget holds ([cli.md](cli.md)).
- A binary MUST refuse a project whose format version is newer than it
  understands, with a diagnostic naming the required version. `REQ-OPS-004 ci`
- A release that changes the format incompatibly MUST ship an automatic
  migration, run as `iron upgrade`, that rewrites the project files to the
  new version and leaves a reviewable Git diff. A format change without a
  migration is a release blocker. `REQ-OPS-005 ci`
- A binary MUST read the previous format version, so a site can upgrade the
  binary first and the project second. `REQ-OPS-006 ci`

## Backup and restore

A site's state is exactly two things: the Git repository (configuration)
and the data directory (history, command journal, adjustments, alarm state,
edge buffer, credentials).

- `iron backup <file>` MUST produce a consistent snapshot of the data
  directory while the site is running, without pausing polling or command
  handling. In plant mode it covers the server's local state and records the
  database's own backup position; the database is backed up with its native
  tools. `REQ-OPS-007 ci`
- A snapshot MUST record the binary version, the format version, and the
  config hash it was taken under. `REQ-OPS-008 ci`
- `iron restore <file>` MUST refuse a snapshot from a newer binary and MUST
  verify the snapshot's integrity before replacing anything. `REQ-OPS-009 ci`
- Restoring MUST NOT rewrite history: the command journal is restored as it
  was and is appended to afterwards, never truncated or edited. `REQ-OPS-010 ci`
- The restore procedure — install the binary, check out the commit, restore
  the snapshot, start — is exercised by an integration test on every
  release.

## Engineering gates

IRON's code is expected to be written mostly by language models. That
changes where quality comes from: not from the author's care, but from what
a change must pass. These gates are requirements on the repository.

- Every requirement in the specs carries an identifier
  ([requirements index](../requirements.md)). A pull request that implements
  a requirement MUST add or extend a test that names the identifier; CI
  reports identifiers no test names. `REQ-OPS-011 ci`
- The conformance vectors in `conformance/` are the executable form of the
  specs. The implementation MUST pass all of them, and a spec change that
  alters behavior MUST change the vectors in the same pull request. `REQ-OPS-012 ci`
- CI MUST run and require: formatting, the linter with warnings as errors,
  all unit, integration, and simulation tests ([testing.md](testing.md)),
  the conformance vectors, and the crate-graph check behind the READ/WRITE
  guarantee ([read-write-separation.md](read-write-separation.md)). `REQ-OPS-013 ci`
- Every crate MUST forbid `unsafe` code. An exception — a driver that needs
  a foreign library — is a named crate, listed in the repository, with its
  own review. `REQ-OPS-014 ci`
- `iron-domain` MUST be fuzzed (expression evaluator, decoders, the alarm
  state machine) and held to a line-coverage floor of 90%, a **target**
  until the first measurement. `REQ-OPS-015 ci`
- Dependencies MUST be pinned by a committed lockfile and checked in CI for
  known advisories and license compatibility. A new dependency is a reviewed
  decision, not a side effect. `REQ-OPS-016 ci`
- An API of a third-party library MUST be verified against that library's
  current documentation for the pinned version. A model's memory of an API
  is not a source. `REQ-OPS-017 ci`
- Release artifacts MUST be reproducible from a tagged commit and published
  with checksums, signatures, and a software bill of materials. `REQ-OPS-018 ci`

What no gate provides: evidence from real hardware. A driver is not
`core`-tier until it has run against a physical device
([extensions.md](extensions.md)), and a release is not "production" until a
real site has run it ([roadmap](../business/roadmap.md)).
