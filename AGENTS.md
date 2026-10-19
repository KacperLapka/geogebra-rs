# AGENTS.md — geogebra-rs

Rust workspace (`api`, `wasm`, Axum backend). Build with `cargo build`, test with `cargo test`.

## BMAD Method (this repo uses it)

- Source of truth for process: `_bmad/config.toml`, skills in `.agents/skills/`, work in `_bmad-output/`.
- OpenCode integration (generated, do not hand-edit unless needed):
  - `.opencode/commands/bmad-*.md` → one slash-command per BMAD skill (e.g. `/bmad-build`, `/bmad-prd`), each loads the matching `@skills/bmad-*` skill.
  - `.opencode/agents/bmad-*.md` → BMAD personas as `subagent`s (e.g. `bmad-agent-dev`/Amelia, `bmad-agent-pm`/John). Ask OpenCode to use them for delegated work.
  - Skills also remain directly loadable via `/skills` or `@skills/<id>`.
- Regenerate pointers after a BMAD reinstall/update if skill descriptions changed; hand-edited files under `.opencode/` are preserved by design.
- BMAD skills run `uv run {project-root}/_bmad/scripts/*.py` helpers — allowed in `opencode.json`, requires `uv` + Python ≥3.11.
- Active initiative: see `_bmad/custom/config.user.toml` (`core.active_initiative`); tickets live under `_bmad-output/<initiative>/tickets/`.
