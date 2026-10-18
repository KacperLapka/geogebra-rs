<script>
  import { onMount } from 'svelte';
  import FormulaBar from './FormulaBar.svelte';
  import Canvas from './Canvas.svelte';
  import Tooltip from './Tooltip.svelte';
  import Sliders from './Sliders.svelte';
  import { getBackend } from './lib/wasm.js';

  let backend = $state(null);
  let formula = $state('a*x^2');
  let error = $state(null);
  let params = $state({});
  let points = $state([]);
  let grid = $state({ x: [], y: [] });
  let viewport = $state({ x_min: -10, x_max: 10, y_min: -10, y_max: 10 });
  let hover = $state(null);
  let workspace = $state(null);

  function refresh() {
    if (!backend) return;
    const sample = backend.samplePoints(240);
    points = sample.points;
    grid = backend.gridLines();
    workspace = backend.serializeWorkspace();
    const known = {};
    for (const name of sample.params ?? []) known[name] = params[name] ?? 1;
    params = known;
  }

  function applyFormula(text) {
    if (!backend) return;
    formula = text;
    const res = backend.setFunction('f', text);
    if (res.ok) {
      error = null;
      const next = { ...params };
      for (const name of res.params) if (!(name in next)) next[name] = 1;
      for (const name of Object.keys(next)) if (!res.params.includes(name)) delete next[name];
      params = next;
    } else {
      error = { message: res.message, span: res.span };
    }
    refresh();
  }

  function handleParam(name, value) {
    if (!Number.isFinite(value)) return;
    backend.updateParam(name, value);
    params = { ...params, [name]: value };
    refresh();
  }

  function handleViewport(vp) {
    viewport = vp;
    backend.updateViewport(vp.x_min, vp.x_max, vp.y_min, vp.y_max);
    refresh();
  }

  function handleHover(px, py) {
    const res = backend.queryHover(px, py);
    hover = { px, py, math_x: res.math_x, math_y: res.math_y };
  }

  onMount(async () => {
    backend = await getBackend();
    backend.setCanvasSize(800, 420);
    applyFormula(formula);
  });
</script>

<main>
  <h1>geogebra-rs — tracer bullet</h1>

  {#if backend}
    <p class="banner {backend.kind}">
      {#if backend.kind === 'rust'}
        Rust-backed math active (wasm-pack bundle loaded).
      {:else}
        Dev fallback math active — run <code>wasm-pack build</code> to link the real Rust bundle.
      {/if}
    </p>
  {:else}
    <p class="banner fallback">Loading math backend…</p>
  {/if}

  <FormulaBar value={formula} {error} onInput={applyFormula} />

  <Canvas
    {points}
    {viewport}
    {grid}
    {hover}
    onViewport={handleViewport}
    onHover={handleHover}
    onLeave={() => (hover = null)}
    onSize={(w, h) => backend?.setCanvasSize(w, h)}
  >
    <Tooltip {hover} />
  </Canvas>

  <Sliders {params} onParam={handleParam} />

  {#if workspace}
    <details class="workspace">
      <summary>workspace JSON (stub, version {workspace.version})</summary>
      <pre>{JSON.stringify(workspace, null, 2)}</pre>
    </details>
  {/if}
</main>
