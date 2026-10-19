<script>
  import { onMount, tick } from 'svelte';
  import FormulaBar from './FormulaBar.svelte';
  import Canvas from './Canvas.svelte';
  import Tooltip from './Tooltip.svelte';
  import Sliders from './Sliders.svelte';
  import StyleControls from './StyleControls.svelte';
  import { getBackend } from './lib/wasm.js';
  import {
    isSlug,
    slugFromLocation,
    shortlink,
    shareUrl,
    saveWorkspace,
    loadWorkspace,
    restoreWorkspace
  } from './lib/share.js';

  let backend = $state(null);
  let formula = $state('a*x^2');
  let error = $state(null);
  let params = $state({});
  // Tangent point `x0`: ordinary session param for the Rust overlay path,
  // never a generic slider (filtered out below so it renders once).
  let x0 = $state(1.0);
  let markers = $state([]);
  let tangent = $state(null);
  let asymptotes = $state([]);
  let curves = $state([]);
  let grid = $state({ x: [], y: [] });
  let viewport = $state({ x_min: -10, x_max: 10, y_min: -10, y_max: 10 });
  let hover = $state(null);
  let workspace = $state(null);
  let gpu = $state({ status: 'cpu-fallback', viewport_revision: 0 });
  let fps = $state(null);
  // Three demo curves (4.3): `f` stays formula-bar-driven; `g`/`h` are fixed
  // demos so the palette/width/visibility polish shows without calculus
  // (derivative math stays with the symbolic-calculus epic).
  const DEMO_CURVES = [
    { id: 'f', formula: 'a*x^2' },
    { id: 'g', formula: '1/x' },
    { id: 'h', formula: 'sin(x)' }
  ];
  // Local fallback when the bridge style map has no entry for an id;
  // render-owned truth lives in the bridge (E4-STYLE-STRUCT).
  const FALLBACK_STYLE = { color: '#2563eb', widthPx: 3.5, dashed: false, visible: true };

  const styleOf = (id) => curves.find((c) => c.id === id)?.style ?? { ...FALLBACK_STYLE };

  function refresh() {
    if (!backend) return;
    const ws = backend.serializeWorkspace();
    workspace = ws;
    grid = backend.gridLines();
    gpu = backend.gpuStatus();
    // Per-id styles from the bridge style map (single source of truth).
    const nextStyles = {};
    for (const [id, entry] of Object.entries(ws.style ?? {})) {
      nextStyles[id] = {
        color: entry?.color_hex ?? FALLBACK_STYLE.color,
        widthPx: entry?.width_px ?? FALLBACK_STYLE.widthPx,
        dashed: entry?.dashed ?? false,
        visible: entry?.visible ?? true
      };
    }
    // Sample every function known to the session, sorted for stable order.
    const ids = Object.keys(ws.functions ?? {});
    ids.sort();
    curves = ids.map((id) => ({
      id,
      points: backend.samplePointsFor(id, 240).points,
      style: nextStyles[id] ?? { ...FALLBACK_STYLE }
    }));
    // Calculus overlays from the Rust `sample_points` (2.6, additive-only):
    // viewport-scoped markers + the `x0` tangent segment + vertical
    // asymptotes (3.1). Stale bundles without the fields read as empty
    // overlays, never a crash.
    try {
      const overlay = backend.samplePoints(240);
      markers = overlay.markers ?? [];
      tangent = overlay.tangent ?? null;
      asymptotes = overlay.asymptotes ?? [];
    } catch {
      markers = [];
      tangent = null;
      asymptotes = [];
    }
    const known = {};
    for (const name of Object.keys(ws.params ?? {})) {
      // `x0` has its dedicated slider below; never a generic slider.
      if (name === 'x0') continue;
      known[name] = params[name] ?? ws.params[name] ?? 1;
    }
    params = known;
    // Keep the dedicated slider in sync when `x0` arrives via restore/share.
    if (Number.isFinite(ws.params?.x0)) x0 = ws.params.x0;
  }

  // Share/load tracer (5.1): Share button + panel + /w/<slug> and ?s=<slug>.
  let shareOpen = $state(false);
  let shareBusy = $state(false);
  let shareSlug = $state(null);
  let shareLink = $state('');
  let shareNote = $state('');
  let shareTone = $state('muted');
  let shareRetry = $state(false);
  let loadNote = $state('');
  let online = $state(typeof navigator !== 'undefined' ? (navigator.onLine ?? true) : true);
  let shareButtonEl = $state(null);
  let panelEl = $state(null);
  let slugInputEl = $state(null);
  let retryEl = $state(null);

  function applyFormula(text) {
    if (!backend) return;
    formula = text;
    const res = backend.setFunction('f', text);
    if (res.ok) {
      error = null;
      const next = { ...params };
      // `x0` keeps its dedicated slider even if a formula names it.
      for (const name of res.params) if (name !== 'x0' && !(name in next)) next[name] = 1;
      for (const name of Object.keys(next)) if (name !== 'x0' && !res.params.includes(name)) delete next[name];
      params = next;
    } else {
      error = { message: res.message, span: res.span };
    }
    refresh();
  }

  function handleParam(name, value) {
    // R2 real-time path: update_param + resample synchronously so the
    // curve follows the slider drag within the same frame (<16ms budget).
    if (!Number.isFinite(value)) return;
    backend.updateParam(name, value);
    params = { ...params, [name]: value };
    refresh();
  }

  function handleX0(value) {
    // Tangent-point path (2.6): same-frame `updateParam("x0")` + resample
    // as the generic sliders, so the tangent follows the drag.
    if (!Number.isFinite(value)) return;
    backend.updateParam('x0', value);
    x0 = value;
    refresh();
  }

  function handleViewport(vp) {
    viewport = vp;
    backend.updateViewport(vp.x_min, vp.x_max, vp.y_min, vp.y_max);
    refresh();
  }

  function handleHover(px, py) {
    if (!Number.isFinite(px) || !Number.isFinite(py)) return;
    const res = backend.queryHover(px, py);
    hover = { px, py, math_x: res.math_x, math_y: res.math_y, marker: res.marker ?? null };
  }

  function handleStyle(id, colorHex) {
    if (!backend || !/^#[0-9a-fA-F]{6}$/.test(colorHex)) return;
    const cur = styleOf(id);
    backend.setFunctionStyle(id, colorHex, cur.widthPx, cur.dashed);
    refresh();
  }

  function handleDashed(id, dashed) {
    if (!backend) return;
    const cur = styleOf(id);
    backend.setFunctionStyle(id, cur.color, cur.widthPx, !!dashed);
    refresh();
  }

  function handleVisibility(id, visible) {
    if (!backend) return;
    backend.setFunctionVisibility(id, !!visible);
    refresh();
  }

  function closeShare() {
    shareOpen = false;
    shareButtonEl?.focus?.();
  }

  function handlePanelKey(e) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      closeShare();
    }
  }

  async function handleShare() {
    if (!backend || shareBusy || !online) return;
    shareOpen = true;
    shareBusy = true;
    shareSlug = null;
    shareLink = '';
    shareNote = 'Saving…';
    shareTone = 'muted';
    shareRetry = false;
    // Focus the panel for keyboard users once it renders.
    queueMicrotask(() => panelEl?.focus?.());
    const doc = backend.serializeWorkspace();
    const res = await saveWorkspace(doc);
    shareBusy = false;
    if (res.ok) {
      shareSlug = res.slug;
      shareLink = shortlink(res.slug);
      shareNote = '';
      queueMicrotask(() => slugInputEl?.select?.());
    } else if (res.error === 'too-large') {
      shareNote = 'Workspace too large to share.';
      shareTone = 'danger';
      // Manual retry: panel stays open with the local workspace untouched;
      // one click re-POSTs the current workspace once it is simplified.
      shareRetry = true;
      tick().then(() => retryEl?.focus?.());
    } else if (res.error === 'network' || !online) {
      shareNote = "You're offline. Showing your local workspace.";
      shareTone = 'muted';
    } else {
      shareNote = 'Could not share this workspace.';
      shareTone = 'danger';
    }
  }

  async function handleCopy() {
    if (!shareSlug) return;
    const full = shareUrl(shareSlug, window.location.origin);
    try {
      await navigator.clipboard.writeText(full);
    } catch {
      // Clipboard blocked (projector rooms, locked-down browsers):
      // fall back to manual selection of the slug field.
      slugInputEl?.select?.();
      return;
    }
    shareNote = 'Link copied.';
    shareTone = 'success';
  }

  async function restoreFromSlug(slug) {
    if (!backend || !isSlug(slug)) return;
    const loaded = await loadWorkspace(slug);
    if (!loaded.ok) {
      if (loaded.error === 'unknown') {
        loadNote = 'Unknown workspace. Check the link and try again.';
      } else if (loaded.error === 'network') {
        loadNote = "You're offline. Showing your local workspace.";
      } else {
        loadNote = 'Could not load this workspace.';
      }
      return;
    }
    const doc = loaded.doc;
    const res = restoreWorkspace(backend, doc);
    if (!res.ok) {
      loadNote = 'Could not load this workspace.';
      return;
    }
    if (typeof doc.functions?.f === 'string') formula = doc.functions.f;
    // `x0` restores to its dedicated slider, never the generic list.
    if (Number.isFinite(doc.params?.x0)) x0 = doc.params.x0;
    params = Object.fromEntries(Object.entries(doc.params ?? {}).filter(([k]) => k !== 'x0'));
    viewport = { ...doc.viewport };
    error = null;
    loadNote = '';
    refresh();
  }

  onMount(async () => {
    backend = await getBackend();
    backend.setCanvasSize(800, 420);
    // Seed the tangent point so the 2.6 overlay draws on first paint;
    // missing `x0` would otherwise render `tangent: null` until first drag.
    backend.updateParam('x0', x0);
    // Shared link view: primary /w/<slug>, fallback ?s=<slug>.
    const slug = slugFromLocation(window.location);
    if (slug) {
      // Link loads reproduce the stored session: never seed demos here;
      // restoreFromSlug sets functions/params/viewport/styles from the doc.
      await restoreFromSlug(slug);
    } else {
      // Fixed demo curves so the 4.3 palette shows three ribbons; `f` flows
      // through the formula bar below. Seeded only without a slug.
      for (const c of DEMO_CURVES) {
        if (c.id !== 'f') backend.setFunction(c.id, c.formula);
      }
      applyFormula(formula);
    }
    const goOnline = () => (online = true);
    const goOffline = () => (online = false);
    window.addEventListener('online', goOnline);
    window.addEventListener('offline', goOffline);
    const onPointer = (e) => {
      if (!shareOpen) return;
      const t = e.target;
      if (panelEl?.contains(t) || shareButtonEl?.contains(t)) return;
      shareOpen = false;
    };
    window.addEventListener('pointerdown', onPointer);
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
      {' '}GPU: {gpu.status}{fps !== null ? ` · ${fps} FPS` : ''} (rev {gpu.viewport_revision}).
    </p>
  {:else}
    <p class="banner fallback">Loading math backend…</p>
  {/if}

  <div class="header-row">
    <div class="formula-wrap">
      <FormulaBar value={formula} {error} onInput={applyFormula} />
    </div>
    <button
      class="share-button"
      bind:this={shareButtonEl}
      disabled={!backend || shareBusy || !online}
      aria-expanded={shareOpen}
      aria-controls="share-panel"
      title={online ? 'Share this workspace' : "You're offline"}
      onclick={shareOpen ? closeShare : handleShare}
      onkeydown={handlePanelKey}
    >
      {shareBusy ? 'Saving…' : 'Share'}
    </button>
  </div>

  {#if shareOpen}
    <div
      id="share-panel"
      class="share-panel"
      bind:this={panelEl}
      tabindex="-1"
      role="dialog"
      aria-label="Share workspace"
      onkeydown={handlePanelKey}
    >
      <label class="share-label" for="slug-field">Workspace link</label>
      <div class="share-row">
        <input
          id="slug-field"
          class="slug-field"
          bind:this={slugInputEl}
          type="text"
          readonly
          value={shareSlug ? shareLink : ''}
          placeholder={shareBusy ? 'Saving…' : '/w/--------'}
          aria-readonly="true"
        />
        <button class="copy-control" disabled={!shareSlug} onclick={handleCopy}>Copy</button>
        {#if shareRetry}
          <button
            class="retry-control"
            bind:this={retryEl}
            disabled={shareBusy || !online}
            onclick={handleShare}
          >Retry</button>
        {/if}
      </div>
      <p class="status-note {shareTone}" aria-live="polite">
        {#if shareBusy}Saving…{:else if shareNote}{shareNote}{:else if shareSlug}Ready — copy the link.{/if}
      </p>
    </div>
  {/if}

  {#if loadNote}
    <p class="status-note danger load-note" aria-live="polite">{loadNote}</p>
  {/if}

  <Canvas
    {curves}
    {viewport}
    {grid}
    {hover}
    {markers}
    {tangent}
    {asymptotes}
    onViewport={handleViewport}
    onHover={handleHover}
    onLeave={() => (hover = null)}
    onSize={(w, h) => backend?.setCanvasSize(w, h)}
    onFps={(value) => (fps = value)}
  >
    <Tooltip {hover} />
  </Canvas>

  {#each curves as curve (curve.id)}
    <StyleControls
      curveId={curve.id}
      colorHex={curve.style.color}
      widthPx={curve.style.widthPx}
      dashed={curve.style.dashed}
      visible={curve.style.visible}
      onStyle={(hex) => handleStyle(curve.id, hex)}
      onDashed={(dashed) => handleDashed(curve.id, dashed)}
      onVisibility={(visible) => handleVisibility(curve.id, visible)}
    />
  {/each}

  <Sliders {params} onParam={handleParam} />

  <div class="sliders x0-slider">
    <div class="slider-row">
      <label for="slider-x0"><code>x0 = {Number(x0).toFixed(2)}</code></label>
      <input
        id="slider-x0"
        type="range"
        min={viewport.x_min}
        max={viewport.x_max}
        step={(viewport.x_max - viewport.x_min) / 400 || 0.01}
        value={x0}
        aria-label="x0 slider"
        title="x0 = {x0}"
        oninput={(e) => handleX0(parseFloat(e.target.value))}
      />
      <input
        type="number"
        step="0.1"
        value={x0}
        aria-label="x0 value"
        oninput={(e) => handleX0(parseFloat(e.target.value))}
      />
    </div>
  </div>

  {#if workspace}
    <!-- Workspace JSON owned shape for Epic 5: {version:1, functions, params, viewport, style}.
         Restored via setFunction + updateParam + updateViewport + the style
         setters (AD-5, no new ABI). -->
    <details class="workspace">
      <summary>workspace JSON (version {workspace.version})</summary>
      <pre>{JSON.stringify(workspace, null, 2)}</pre>
    </details>
  {/if}
</main>
