# cargo-rail Planning Integration

## Intent
This repo is best served by `cargo rail plan` + `cargo xtask` execution.
The `.config/rail.toml` includes custom surfaces for themes, queries, and docs.

## Local developer flow
```bash
cargo rail config validate --strict
cargo rail plan --merge-base --explain
cargo rail plan --merge-base -f json
```

## GitHub Actions integration (cargo-rail-action)
Use planner outputs to gate existing lanes.

```yaml
- uses: loadingalias/cargo-rail-action@v3
  id: rail

- name: Rust tests
  if: steps.rail.outputs.test == 'true'
  run: cargo test --workspace

- name: Theme check
  if: steps.rail.outputs.themes == 'true'
  run: cargo xtask theme-check

- name: Query check
  if: steps.rail.outputs.queries == 'true'
  run: cargo xtask query-check
```

## UI output that teams should read
- action summary surface table and reasons
- `steps.rail.outputs.plan-json` for custom job logic
- `cargo rail plan --explain` as the local CI mirror

## Measured impact (last 20 commits)
- Could skip build: 30%
- Could skip tests: 30%
- Targeted (not full run): 40%
