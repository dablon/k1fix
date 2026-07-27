# Fixtures

Example meshes for manual / E2E checks. Regenerate with:

```bash
cargo run --example gen_fixtures
```

| File | Purpose |
|------|---------|
| `cube.stl` / `cube.3mf` / `cube.step` | Closed unit cube |
| `tray.stl` | Oversized tray that autofit must rotate onto the K1 bed |
| `open_cube.stl` | Cube missing the top face (MESH001) |
