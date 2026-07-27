# k1fix

CLI en Rust para **diagnosticar, reparar y encajar** modelos 3D (STL / 3MF / STEP faceted) en impresoras Creality **K1 / K1C / K1 Max**.

## Arquitectura (Clean Architecture)

```
presentation  →  application  →  domain
      ↓               ↓
infrastructure ───────┘
```

| Capa | Ruta | Responsabilidad |
|------|------|-----------------|
| Domain | `src/domain/` | Malla, topología, repair, autofit, diagnósticos, perfiles — **cero IO** |
| Application | `src/application/` | Use cases + ports (`MeshLoader`, `Fs`, …) |
| Infrastructure | `src/infrastructure/` | Adapters STL/3MF/STEP, filesystem, `MemFs` |
| Presentation | `src/presentation/` | CLI clap (composition root) |

ADR: [docs/adr/0001-clean-architecture.md](docs/adr/0001-clean-architecture.md)

## Uso rápido

```bash
cargo build --release
./target/release/k1fix profiles list
./target/release/k1fix inspect fixtures/tray.stl --profile k1 --json report.json
./target/release/k1fix fix fixtures/tray.stl -o out.stl --profile k1
./target/release/k1fix fix fixtures/tray.stl -o out.stl --scale-to-fit
./target/release/k1fix convert fixtures/cube.step -o cube.3mf
```

### Códigos de salida

| Código | Significado |
|--------|-------------|
| 0 | Limpio / solo info |
| 1 | Warnings (overhangs, margen, etc.) |
| 2 | Errores / no cabe en el volumen |
| 3 | Fallo de IO / parseo / perfil |

### Diagnósticos

- **FIT001–FIT004** — cama / altura / flotando / margen  
- **MESH001–MESH007** — agujeros, non-manifold, winding, degenerados, duplicados, shells, auto-intersección  
- **PRT001–PRT005** — paredes finas, overhangs, detalle, escala, demasiados triángulos  

## Docker

```bash
just test
just lint
just e2e
just build
docker build --target runtime -t k1fix .
docker run --rm -v "${PWD}:/work" k1fix inspect /work/fixtures/tray.stl
```

## Desarrollo

```bash
cargo test --tests
cargo clippy --all-targets -- -D warnings
cargo run --example gen_fixtures
just host-cov          # requiere cargo-llvm-cov; gate 80% líneas
```

Rust **1.92** (`rust-toolchain.toml`).

## Fixtures

Generados con `cargo run --example gen_fixtures`:

- `fixtures/cube.stl` / `.3mf` / `.step`
- `fixtures/tray.stl` — caso Küchenablage (221.64×239.64×40)
- `fixtures/open_cube.stl` — agujero para MESH001

## Licencia

MIT
