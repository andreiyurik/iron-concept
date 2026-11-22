---
status: accepted
type: decision
---

# ADR 0013 — Git Is the Source of Truth, With Bounded Runtime Adjustments

## Context

"Config is text in Git" is an IRON invariant, and `iron diff` exists to
prove that a plant runs what Git says. But the specs had no answer for the
most ordinary event on a plant floor: someone on site, at night, changes an
alarm limit because it is chattering. The `iron diff` example even showed
such a change ("deployed … not committed") without any spec describing how
it could have been made.

A system with no sanctioned path for this does not prevent the change. It
moves the change somewhere worse — an edited file on the target, a disabled
alarm, a notification muted on a phone — and the drift detector then reports
a violation of a rule the product made impossible to follow.

## Decision

Git stays the only source of configuration. On top of it, a running site
may hold **adjustments**: a deliberately tiny set of alarm fields (limit,
hysteresis, delays) that an engineer can change live, each one validated,
journaled, and attributed. What runs is the *effective* configuration —
committed config plus adjustments
([specs/config-lifecycle.md](../specs/config-lifecycle.md)).

The layer is bounded on every side:

- It can change values of existing alarm fields. It cannot create, remove,
  or re-target anything, and it cannot touch commands.
- `iron diff` reports adjustments as attributed, expected differences —
  distinct from unexplained drift, which stays an error.
- `iron diff --pull` writes them back into the spec files for review.
- `iron deploy` refuses to overwrite adjustments it does not contain.

## Alternatives

- **Pure GitOps: no runtime changes at all** — rejected. Correct for a web
  service, wrong for a plant, where the person with the knowledge is at the
  panel and the person with repository access is asleep. It would be
  bypassed on the first night.
- **Free runtime editing with optional export** (the classical SCADA model)
  — rejected. This is the condition `iron diff` exists to end: the running
  system becomes the truth and the files become a suggestion.
- **The server commits to Git itself** — rejected for v1. It places
  repository write credentials on a plant host, makes the plant depend on
  reaching a Git remote, and turns a merge conflict into an operations
  problem. Pulling adjustments from a workstation keeps credentials and
  review where they already are.
- **Adjust anything, not just alarm tuning** — rejected. Every field added
  to the adjustable set is a field that can differ from Git. Alarm tuning is
  the case with real night-time urgency; scaling and addresses are
  engineering changes that deserve review.

## Trade-offs

- "The plant runs exactly what Git says" becomes "exactly what Git says,
  plus a listed set of attributed adjustments". The stronger sentence was
  simpler and was not going to be true.
- The config hash now covers adjustments, so a site with adjustments does
  not hash-match its commit. Tooling must show both.
- Refusing a deploy over un-pulled adjustments adds a step at the worst
  moment for patience. The alternative is silently reverting someone's
  tuning.
- The adjustable set will be argued over. Tag deadbands and scan rates are
  plausible candidates that are deliberately out.

## Revisit when

A real site shows a field outside the adjustable set being changed by
workaround more than once — that field is a candidate. Or a site operates
with Git reachable from the plant and asks for the server to open the pull
request itself.
