<script>
  import { onMount } from 'svelte';
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
  let points = $state([]);
  let grid = $state({ x: [], y: [] });
  let viewport = $state({ x_min: -10, x_max: 10, y_min: -10, y_max: 10 });
  let hover = $state(null);
  let workspace = $state(null);
  let gpu = $state({ status: 'cpu-fallback', viewport_revision: 0 });
  let fps = $state(null);
  // Render-owned style mirror for the tracer curve `f`: hex stays local for
  // the 2D stroke, width/dashed/visibility sync from the bridge return values.
  let style = $state({ color: '#2563eb', widthPx: 3.5, dashed: false, visible: true });

  // Share/load tracer (5.1): Share button + panel + /w/<slug> and ?s=<slug>.
  let shareOpen = $state(false);
  let shareBusy = $state(false);
  let shareSlug = $state(null);
  let shareLink = $state('');
  let shareNote = $state('');
  let shareTone = $state('muted');
  let loadNote = $state('');
  let online = $state(typeof navigator !== 'undefined' ? (navigator.onLine ?? true) : true);
  let shareButtonEl = $state(null);
  let panelEl = $state(null);
  let slugInputEl = $state(null);

  function refresh() {
    if (!backend) return;
    const sample = backend.samplePoints(240);
    points = sample.points;
    grid = backend.gridLines();
    workspace = backend.serializeWorkspace();
    gpu = backend.gpuStatus();
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
    // R2 real-time path: update_param + resample synchronously so the
    // curve follows the slider drag within the same frame (<16ms budget).
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
    if (!Number.isFinite(px) || !Number.isFinite(py)) return;
    const res = backend.queryHover(px, py);
    hover = { px, py, math_x: res.math_x, math_y: res.math_y, marker: res.marker ?? null };
  }

  function handleStyle(colorHex) {
    if (!backend || !/^#[0-9a-fA-F]{6}$/.test(colorHex)) return;
    const res = backend.setFunctionStyle('f', colorHex, style.widthPx, style.dashed);
    style = { color: colorHex, widthPx: res?.width_px ?? style.widthPx, dashed: res?.dashed ?? style.dashed, visible: res?.visible ?? style.visible };
  }

  function handleDashed(dashed) {
    if (!backend) return;
    const res = backend.setFunctionStyle('f', style.color, style.widthPx, !!dashed);
    style = { ...style, widthPx: res?.width_px ?? style.widthPx, dashed: res?.dashed ?? !!dashed };
  }

  function handleVisibility(visible) {
    if (!backend) return;
    const res = backend.setFunctionVisibility('f', !!visible);
    style = { ...style, visible: res?.visible ?? !!visible };
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
    params = { ...doc.params };
    viewport = { ...doc.viewport };
    error = null;
    loadNote = '';
    refresh();
  }

  onMount(async () => {
    backend = await getBackend();
    backend.setCanvasSize(800, 420);
    applyFormula(formula);
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
    // Shared link view: primary /w/<slug>, fallback ?s=<slug>.
    const slug = slugFromLocation(window.location);
    if (slug) await restoreFromSlug(slug);
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
    {points}
    {viewport}
    {grid}
    {hover}
    {style}
    onViewport={handleViewport}
    onHover={handleHover}
    onLeave={() => (hover = null)}
    onSize={(w, h) => backend?.setCanvasSize(w, h)}
    onFps={(value) => (fps = value)}
  >
    <Tooltip {hover} />
  </Canvas>

  <StyleControls
    colorHex={style.color}
    widthPx={style.widthPx}
    dashed={style.dashed}
    visible={style.visible}
    onStyle={handleStyle}
    onDashed={handleDashed}
    onVisibility={handleVisibility}
  />

  <Sliders {params} onParam={handleParam} />

  {#if workspace}
    <!-- Workspace JSON owned shape for Epic 5: {version:1, functions, params, viewport}.
         Restored via setFunction + updateParam + updateViewport (AD-5, no new ABI). -->
    <details class="workspace">
      <summary>workspace JSON (version {workspace.version})</summary>
      <pre>{JSON.stringify(workspace, null, 2)}</pre>
    </details>
  {/if}
</main>
