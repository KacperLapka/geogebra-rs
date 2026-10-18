- source_plan: `/home/kacper/Development/rs/geogebra-rs/_bmad-output/initiative-geogebra-rs/tickets/epic-rendering-polish/tickets/story-tracer-bullet-styled-ribbon-with-shader-contract-plan.md`
  summary: Reconcile Axum flat workspace validation with pre-existing wrapped SaveBody callers.
  evidence: api/routes save now requires flat {version:1,functions,params,viewport} and returns 400 for the prior wrapped {version,workspace} shape; change is sibling 5.1 work in the same diff since baseline, not 4.1, so it defers to the persistence entry.
