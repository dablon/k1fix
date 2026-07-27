# k1fix

CLI en Rust para **diagnosticar, reparar y encajar** mallas 3D en impresoras Creality **K1 / K1C / K1 Max**.

Cuando Orca/Bambu te escupen un STL por pasarse de la cama (`221×239` en un bed `220×220`), `k1fix` te dice *por qué*, repara lo que puede y, si hace falta, rota/escala para que quepa — **preferiendo impresión plana** (bajo Z), no torres de 220 mm de alto.

| Formato | Lectura | Escritura |
|---------|---------|-----------|
| STL | sí | sí |
| 3MF | sí | sí |
| STEP | sí (faceted / ligero) | vía `convert` a STL/3MF |

> STEP no es un tessellator BREP de camión: espera geometría facetada razonable. Para CAD pesado, exportá STL/3MF desde tu CAD y seguí.

## Instalación

Requiere [Rust](https://rustup.rs/) **1.92** (`rust-toolchain.toml` lo fija). Si no hay Rust, los scripts de abajo intentan instalar **rustup**.

### Scripts (recomendado)

Desde el repo clonado:

**Linux / macOS**

```bash
chmod +x scripts/install.sh
./scripts/install.sh
```

Instala en `~/.local/bin` (override: `K1FIX_INSTALL_DIR`).

**Windows (PowerShell)**

```powershell
Set-ExecutionPolicy -Scope CurrentUser RemoteSigned   # una vez, si hace falta
.\scripts\install.ps1
```

Instala en `%LOCALAPPDATA%\k1fix\bin` y lo agrega al PATH de usuario. Abrí una terminal nueva después.

Variables opcionales (ambos):

| Variable | Default | Uso |
|----------|---------|-----|
| `K1FIX_REPO_URL` | `https://github.com/k1fix/k1fix.git` | Clone si no hay repo local |
| `K1FIX_INSTALL_DIR` | `~/.local/bin` / `%LOCALAPPDATA%\k1fix\bin` | Destino del binario |
| `K1FIX_BRANCH` | `master` | Branch a clonar |

```bash
# ejemplo: otro mirror / branch
K1FIX_REPO_URL=https://github.com/<user>/k1fix.git K1FIX_BRANCH=main ./scripts/install.sh
```

```powershell
.\scripts\install.ps1 -RepoUrl 'https://github.com/<user>/k1fix.git' -Branch main
```

### Manual

```bash
git clone https://github.com/<user>/k1fix.git
cd k1fix
cargo build --release
./target/release/k1fix --help
# o: cargo install --path . --force
```

### Docker

```bash
docker build --target runtime -t k1fix .
docker run --rm -v "${PWD}:/work" -w /work k1fix inspect fixtures/tray.stl
```

## Uso rápido

```bash
# ¿Cabe? ¿está roto?
k1fix inspect modelo.stl --profile k1

# Reparar + autofit (plano por defecto)
k1fix fix modelo.stl -o out.stl --profile k1

# Plano pero el XY no entra → escala
k1fix fix modelo.stl -o out.stl --profile k1 --scale-to-fit

# Querés la torre vertical (contact-first)
k1fix fix modelo.stl -o out.stl --prefer-tall

# Solo diagnóstico en JSON
k1fix inspect modelo.stl --json report.json

# Convertir formatos
k1fix convert cube.step -o cube.3mf
k1fix profiles list
```

### Ejemplo real (Küchenablage)

Original rechazado por el slicer ≈ **221.64 × 239.64 × 40 mm** en bed K1 **220×220**.

```bash
k1fix fix Küchenablage.stl -o Küchenablage-fixed.stl --profile k1 --scale-to-fit
```

Resultado típico: **~198 × 214 × 36 mm** (plano, ~89 % de escala). Sin `--scale-to-fit` puede caber **vertical** (~175×176×222) — imprimible, pero una mierda de soportes.

## Comandos

### `inspect`

Inspecciona sin escribir malla.

```text
k1fix inspect <FILE> [--profile k1|k1c|k1max] [--margin 3] [--json PATH] [--tess-tol 0.05]
```

### `fix`

Repara y/o auto-encaja. **Requiere** `-o/--output`.

| Flag | Default | Qué hace |
|------|---------|----------|
| `--profile` | `k1` | Perfil de impresora |
| `--margin` | `3` | Margen de cama (mm) |
| `--scale-to-fit` | off | Escala uniforme si no cabe (útil para flat) |
| `--prefer-tall` | off | Prefiere orientación alta; **default es flat/low-Z** |
| `--no-autofit` | off | Solo repair |
| `--no-repair` | off | Solo autofit |
| `--drop-specks` | off | Elimina shells diminutos |
| `--dry-run` | off | Corre pipeline sin escribir archivo |
| `--json` | — | Reporte JSON de salida |
| `--tess-tol` | `0.05` | Tolerancia STEP |

### `convert`

```text
k1fix convert <INPUT> -o <OUTPUT> [--tess-tol 0.05]
```

Extensiones soportadas: `.stl`, `.3mf`, `.step` / `.stp`.

### `profiles list`

Lista perfiles embebidos.

## Perfiles

| ID | Impresora | Volumen (mm) |
|----|-----------|--------------|
| `k1` | Creality K1 | 220 × 220 × 250 |
| `k1c` | Creality K1C | 220 × 220 × 250 |
| `k1max` | Creality K1 Max | 300 × 300 × 300 |

Definidos en `profiles/*.toml` (bed, Z, boquilla, layer height, margen).

## Autofit (cómo piensa)

1. Genera candidatos axis-aligned (24) + PCA.
2. En camas cuadradas prueba yaw para maximizar margen.
3. Coloca la pieza sobre la cama (Z≥0).
4. Con **`prefer_flat` (default)**: prioriza **altura Z baja** entre los que caben.
5. Si un upright sin escala cabe pero el flat necesita escala y pasaste `--scale-to-fit`, elige el **flat escalado** cuando la altura queda &lt; ~50 % de la torre.
6. `--prefer-tall` invierte esa preferencia (modo “máximo contacto / torre”).

Repair pipeline (resumen): weld → degenerates → non-manifold → orient → hole fill → duplicates → specks → normals.

## Códigos de salida

| Código | Significado |
|--------|-------------|
| **0** | Limpio / solo info |
| **1** | Warnings (overhangs, margen, etc.) |
| **2** | Errores de malla / no cabe (o autofit imposible sin split/scale) |
| **3** | Fallo de IO / parseo / perfil |

Usá el código en CI: `k1fix inspect foo.stl || echo $?`.

## Diagnósticos

### FIT — volumen de impresión

| ID | Tema |
|----|------|
| FIT001 | No cabe en XY (cama) |
| FIT002 | Excede altura Z |
| FIT003 | Flotando / no apoyado |
| FIT004 | Margen de cama insuficiente |

### MESH — integridad

| ID | Tema |
|----|------|
| MESH001 | Agujeros (open boundary edges) |
| MESH002 | Non-manifold |
| MESH003 | Winding inconsistente |
| MESH004 | Triángulos degenerados |
| MESH005 | Caras duplicadas |
| MESH006 | Shells desconectados |
| MESH007 | Auto-intersección |

### PRT — imprimibilidad

| ID | Tema |
|----|------|
| PRT001 | Paredes finas |
| PRT002 | Overhangs &gt; 45° |
| PRT003 | Detalle &lt; layer height |
| PRT004 | Escala sospechosa |
| PRT005 | Demasiados triángulos |

## Arquitectura

Clean Architecture en **un solo crate** (sin workspace de `crates/`):

```
presentation  →  application  →  domain
      ↓               ↓
infrastructure ───────┘
```

| Capa | Ruta | Responsabilidad |
|------|------|-----------------|
| Domain | `src/domain/` | Malla, topología, repair, autofit, diagnósticos, perfiles — **cero IO** |
| Application | `src/application/` | Use cases (`inspect` / `fix` / `convert`) + ports |
| Infrastructure | `src/infrastructure/` | STL / 3MF / STEP, filesystem, `MemFs`, progreso |
| Presentation | `src/presentation/` | CLI clap (composition root) |

ADR: [docs/adr/0001-clean-architecture.md](docs/adr/0001-clean-architecture.md)

## Desarrollo

```bash
cargo test                  # unit + lib
cargo test --tests          # e2e / contracts / use_cases
cargo clippy --all-targets -- -D warnings
cargo run --example gen_fixtures
```

Con [just](https://github.com/casey/just) + Docker Compose:

```bash
just test      # tests en container
just lint
just e2e
just cov       # cobertura en Docker
just build
just host-cov  # host: cargo-llvm-cov, gate 80% líneas
```

Gate de cobertura CI: **≥ 80 %** líneas (ignora `main.rs` / examples).

## Fixtures

Generados con `cargo run --example gen_fixtures`:

| Archivo | Para qué |
|---------|----------|
| `fixtures/cube.stl` / `.3mf` / `.step` | smoke / convert |
| `fixtures/tray.stl` | caso tipo Küchenablage (221.64×239.64×40) |
| `fixtures/open_cube.stl` | agujero → MESH001 |

## Limitaciones (v1)

- No **split** de piezas que no caben ni escalando.
- Hole-fill no cierra todos los agujeros de mallas basura del mundo real.
- STEP facetado ≠ OpenCascade completo.
- Autofit es axis-aligned / PCA + yaw; no magia de nesting 3D libre.

## Licencia

[MIT](LICENSE)
