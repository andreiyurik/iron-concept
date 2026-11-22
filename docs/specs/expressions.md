---
status: draft
type: spec
depends-on: [tag-model]
---

# Expressions

IRON has exactly one expression language. It is small, pure, and total, and
it is used everywhere a spec needs a computed value. Rationale and rejected
alternatives: [ADR 0012](../decisions/0012-one-expression-language.md).

The rule this document exists to enforce: **no spec may introduce a second
mini-language.** If a YAML field needs arithmetic or a condition, it uses
this one.

## Where expressions appear

| Context | Field | Evaluated | Operands | Result |
|---|---|---|---|---|
| Expansion | `{…}` inside a device type ([device-types.md](device-types.md)) | Once, by `iron-core`, when instances are expanded | Type parameters, literals | number, or text substituted into the surrounding string |
| Derived tag | `derived:` on a tag ([tag-model.md](tag-model.md)) | At runtime, whenever an input changes | Tags, literals | number or bool — becomes the tag's value |
| Alarm suppression | `suppressed_when:` on an alarm ([alarm-engine.md](alarm-engine.md)) | At runtime, by the server | Tags, literals | bool |

Nothing else evaluates expressions in v1. Dashboards, mimics, and widgets do
not: a widget renders values, it never computes them
([visual-system.md](visual-system.md)).

## The language

```
expr     = or ;
or       = and { "||" and } ;
and      = cmp { "&&" cmp } ;
cmp      = sum [ ( "==" | "!=" | "<" | "<=" | ">" | ">=" ) sum ] ;
sum      = term { ( "+" | "-" ) term } ;
term     = unary { ( "*" | "/" | "%" ) unary } ;
unary    = [ "-" | "!" ] primary ;
primary  = number | "true" | "false" | reference
         | function "(" expr { "," expr } ")" | "(" expr ")" ;
number   = decimal | "0x" hex ;
reference = name { "." name } ;          (* a tag path, or a type parameter *)
```

Two value types: **number** (64-bit float) and **bool**. There are no
strings, no null, no implicit conversions between the two.

Built-in functions — the complete list:

| Function | Meaning |
|---|---|
| `abs(x)` `min(a, b, …)` `max(a, b, …)` `clamp(x, lo, hi)` | Magnitude and bounds |
| `round(x)` `floor(x)` `ceil(x)` | Rounding |
| `sqrt(x)` `pow(x, y)` `exp(x)` `ln(x)` `log10(x)` | Elementary math |
| `if(cond, a, b)` | Selection; `a` and `b` MUST have the same type `REQ-EXP-001 unit` |

There are no user-defined functions, no variables, no loops, no state, no
access to time, history, or I/O.

## Normative rules

- **Static typing.** `iron validate` MUST type-check every expression against
  the declared types of the tags and parameters it references and reject
  mismatches (`bool + number`, a number where `suppressed_when` needs a bool). `REQ-EXP-002 unit`
- **References are full paths.** A runtime expression names tags by their
  canonical dotted path (`zone_1.air_temperature`), never by a relative or
  shortened name. One identity per tag keeps rename and search exact.
- **Unknown references MUST be reported as errors**, with the standard diagnostic
  format ([agent-interface.md](agent-interface.md)). `REQ-EXP-005 unit`
- **No cycles.** Derived tags MAY reference other derived tags; the
  dependency graph MUST be acyclic, and evaluation follows its topological
  order. `REQ-EXP-003 unit`
- **Total and bounded.** Evaluation always terminates in time proportional to
  the expression size. The validator rejects expressions nested deeper than
  64 levels or longer than 512 tokens — a flat chain of operators is as
  deep a tree as nesting is.
- **Arithmetic MUST NOT panic and MUST NOT yield a non-finite value.** Division
  by zero, `sqrt` of a negative, `ln`/`log10` of a non-positive number, and
  any result that is NaN or infinite produce *no value*: the derived tag's
  quality becomes `BAD`; a `suppressed_when` that cannot be evaluated is
  `false`. `REQ-EXP-006 unit`
- **One evaluator.** The implementation lives in `iron-domain` and is the
  same code in the validator, the simulator, the edge agent, and the server.

### Quality and time at runtime

- The quality of a derived value MUST be the **worst quality among all tags the
  expression references** — including both branches of an `if`. An
  expression cannot launder a `BAD` input into a `GOOD` result. `REQ-EXP-007 unit`
- The timestamp of a derived value is the timestamp of the input sample that
  triggered the evaluation.
- `suppressed_when` MUST fail toward annunciation: if any referenced tag is not
  `GOOD`, the condition is `false` and the alarm is **not** suppressed. A
  broken sensor must never silence an alarm. `REQ-EXP-008 unit`

### Expansion context

Inside a device type, `{…}` is evaluated once at expansion time with type
parameters as the only names. The result replaces the braces. Two phases,
never mixed:

```yaml
# in a device type
derived: "{instance}.current / {rated_current} * 100"
# after expansion for pump_03 (rated_current: 18.0)
derived: "pump_03.current / 18.0 * 100"      # now a runtime expression over tags
```

Where an expanded value is used as an address or a count, it MUST be an
integer; `iron validate` rejects `{base + 0.5}` in a register address. `REQ-EXP-004 unit`

## What the language deliberately lacks

- **State and time**: rate of change, moving averages, totalizers, timers,
  "previous value". These need memory across evaluations and well-defined
  behavior across restarts and gaps; they are *deferred* with their trigger
  in [business/deferred.md](../business/deferred.md).
- **Unit algebra.** `unit` on a derived tag is declared by its author; the
  validator does not check that kPa was computed from °C correctly.
- **Side effects.** An expression cannot name a command. There is no
  expression context on the WRITE path
  ([read-write-separation.md](read-write-separation.md)).

When a computation outgrows this language, it is a program, and programs are
plugins ([extensions.md](extensions.md)).
