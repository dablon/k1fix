# ADR 0001 — Clean Architecture layers

## Status

Accepted

## Context

The first scaffold dumped every module flat under `src/`. That fights SOLID
(especially DIP/SRP) and makes IO leak into business rules.

## Decision

Single crate `k1fix` with four layers; dependencies point **inward only**:

```
presentation  →  application  →  domain
       ↓              ↓
infrastructure ───────┘
```

| Layer | Responsibility | May depend on |
|-------|----------------|---------------|
| `domain/` | Entities, mesh/topology/repair/autofit/diagnostics rules, errors | nothing outside domain |
| `application/` | Use cases (`Inspect`, `Fix`, `Convert`) + ports (`Fs`, `MeshLoader`, `MeshStore`, …) | `domain` only |
| `infrastructure/` | STL/3MF/STEP adapters, filesystem, progress | `application` ports + `domain` |
| `presentation/` | Clap CLI; wires adapters into use cases | all layers (composition root) |

## Consequences

- Adding a format = new adapter under `infrastructure/io`, no use-case edits.
- Adding a diagnostic = new `Check` in `domain/diagnostics`, register it.
- Tests inject fake `Fs` / `MeshLoader` without touching disk.
- `main.rs` stays a one-liner.
