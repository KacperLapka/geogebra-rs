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
  import {
    validateOriginal,
    normalizeForBackend,
    deriveAll,
    format3,
    behaviorWord
  } from './lib/derive.js';

  const DESIGN = {
    f: { color: '#4F46E5', width: 2.5, dashed: false },
    fp: { color: '#1FA36B', width: 2.5, dashed: true },
    fpp: { color: '#9333EA', width: 2.5, dashed: true }
  };
  const DEFAULT_VIEW = { x_min: -5, x_max: 5, y_min: -5, y_max: 5 };

  let backend = $state(null);
  let draft = $state('');
  let committed = $state('');
  let error = $state(null);
  let analyzing = $state(false);
  let hasAnalyzed = $state(false);
  let params = $state({});
  let x0 = $state(1.0);
  let markers = $state([]);
  let tangent = $state(null);
  let asymptotes = $state([]);
  let curves = $state([]);
  let grid = $state({ x: [], y: [], x_minor: [], y_minor: [], x_labels: [], y_labels: [], step_x: 0, step_y: 0 });
  let viewport = $state({ ...DEFAULT_VIEW });
  let hover = $state(null);
  let workspace = $state(null);
  let gpu = $state({ status: 'cpu-fallback', viewport_revision: 0 });
  let fps = $state(null);
  let fPrimeDisplay = $state('');
  let fDoubleDisplay = $state('');
  let toggles = $state({ f: true, fp: true, fpp: true, tangent: true, markers: true });
  let announced = $state({ x0: 1, value: '—', slope: '—', behavior: '—' });

  let shareBusy = $state(false);
  let shareSlug = $state(null);
  let shareLink = $state('');
  let shareNote = $state('');
  let shareTone = $state('muted');
  let shareRetry = $state(false);
  let loadNote = $state('');
  let online = $state(typeof navigator !== 'undefined' ? (navigator.onLine ?? true) : true);
  let slugInputEl = $state(null);

  const isStale = () => !hasAnalyzed || draft.trim() !== committed.trim() || !!error || analyzing;
  const showBadge = () => hasAnalyzed && !isStale();

  function refresh() {
    if (!backend) return;
    const ws = backend.serializeWorkspace();
    workspace = ws;
    grid = backend.gridLines();
    gpu = backend.gpuStatus();
    const nextStyles = {};
    for (const [id, entry] of Object.entries(ws.style ?? {})) {
      nextStyles[id] = {
        color: entry?.color_hex ?? '#4f46e5',
        widthPx: entry?.width_px ?? 2.5,
        dashed: entry?.dashed ?? id !== 'f',
        visible: entry?.visible ?? toggles[id] ?? true
      };
    }
    const wantIds = ['f', 'fp', 'fpp'].filter((id) => ws.functions?.[id] !== undefined);
    curves = wantIds.map((id) => ({
      id,
      points: backend.samplePointsFor(id, 240).points,
      style: nextStyles[id] ?? {
        color: DESIGN[id]?.color ?? '#4f46e5',
        widthPx: DESIGN[id]?.width ?? 2.5,
        dashed: DESIGN[id]?.dashed ?? id !== 'f',
        visible: toggles[id] ?? true
      }
    }));
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
      if (name === 'x0') continue;
      known[name] = params[name] ?? ws.params[name] ?? 1;
    }
    params = known;
    if (Number.isFinite(ws.params?.x0)) x0 = ws.params.x0;
    // Sync toggles from restored style visibilities (share restore path).
    for (const id of ['f', 'fp', 'fpp']) {
      if (typeof nextStyles[id]?.visible === 'boolean') toggles[id] = nextStyles[id].visible;
    }
  }

  function validateDraft(text) {
    if (!text.trim()) {
      error = null;
      return;
    }
    const t0 = performance.now();
    const res = validateOriginal(text);
    const dt = performance.now() - t0;
    // Budget guard: parser is sync and far under 50ms; measured for the spike note.
    if (dt > 50) console.warn(`validate took ${dt.toFixed(1)}ms`);
    if (res.ok) error = null;
    else error = { message: res.message, span: res.span };
  }

  function handleDraftInput(text) {
    draft = text;
    validateDraft(text);
  }

  function handleClear() {
    draft = '';
    error = null;
    // Empty state: clear analysis so share row, badge, and derivative card
    // hide (cold); canvas keeps last good curves until next Analyze.
    committed = '';
    hasAnalyzed = false;
    fPrimeDisplay = '';
    fDoubleDisplay = '';
    shareSlug = null;
    shareLink = '';
    shareNote = '';
  }

  async function handleAnalyze(presetFormula) {
    const input = (presetFormula ?? draft).trim();
    if (!input) return;
    if (presetFormula !== undefined) {
      draft = presetFormula;
      validateDraft(presetFormula);
      if (error) return;
    } else if (error) {
      return;
    }
    if (!backend || analyzing) return;
    analyzing = true;
    // Let "Analyzing…" paint; previous curves stay visible (no flash).
    await new Promise((r) => setTimeout(r, 0));
    try {
      let normalized;
      let derived;
      try {
        derived = deriveAll(input);
        normalized = derived.normalized;
      } catch (e) {
        error = { message: e.message ?? 'Parse error', span: e.span ?? [0, input.length] };
        analyzing = false;
        return;
      }
      const res = backend.setFunction('f', normalized);
      if (!res.ok) {
        // Backend spans index the normalized string; when normalization
        // reshaped the typed text the caret would mislead, so omit it.
        error = normalized !== input
          ? { message: res.message, span: null }
          : { message: res.message, span: res.span };
        analyzing = false;
        return;
      }
      // Symbolic only on Analyze: derive f'/f'' once, install as auto curves
      // via existing setFunction so sampling stays in WASM (no new ABI).
      try {
        backend.setFunction('fp', derived.primeBackend);
      } catch {
        // Additive: a derived install failure never blocks the primary curve.
      }
      try {
        backend.setFunction('fpp', derived.doubleBackend);
      } catch {
        // Same: f'' is additive.
      }
      // Fixed DESIGN palette (wins over fallback blue).
      try {
        backend.setFunctionStyle('f', DESIGN.f.color, DESIGN.f.width, DESIGN.f.dashed);
        backend.setFunctionStyle('fp', DESIGN.fp.color, DESIGN.fp.width, DESIGN.fp.dashed);
        backend.setFunctionStyle('fpp', DESIGN.fpp.color, DESIGN.fpp.width, DESIGN.fpp.dashed);
        // Re-assert visibilities from toggles (canvas-only, no analysis refire).
        backend.setFunctionVisibility('f', toggles.f !== false);
        backend.setFunctionVisibility('fp', toggles.fp !== false);
        backend.setFunctionVisibility('fpp', toggles.fpp !== false);
      } catch {
        // Style setters are additive; ignore.
      }
      const next = { ...params };
      for (const name of res.params) if (name !== 'x0' && !(name in next)) next[name] = 1;
      for (const name of Object.keys(next)) if (name !== 'x0' && !res.params.includes(name)) delete next[name];
      params = next;
      // Store normalized so reopen (which restores normalized) matches.
      committed = normalized;
      draft = normalized;
      fPrimeDisplay = derived.primeDisplay;
      fDoubleDisplay = derived.doubleDisplay;
      error = null;
      hasAnalyzed = true;
      // Reset share row for the fresh analysis (gated until success).
      shareSlug = null;
      shareLink = '';
      shareNote = '';
      shareRetry = false;
      refresh();
      commitAnnounced();
    } finally {
      analyzing = false;
    }
  }

  function handleTry(formula) {
    handleAnalyze(formula);
  }

  function handleParam(name, value) {
    if (!Number.isFinite(value)) return;
    backend.updateParam(name, value);
    params = { ...params, [name]: value };
    refresh();
  }

  function handleParamCommit() {
    commitAnnounced();
  }

  function handleX0(value) {
    if (!Number.isFinite(value)) return;
    backend.updateParam('x0', value);
    x0 = value;
    refresh();
  }

  function handleX0Commit(value) {
    if (Number.isFinite(value)) {
      x0 = value;
      commitAnnounced();
    }
  }

  function handleViewport(vp) {
    const { x_min, x_max, y_min, y_max } = vp ?? {};
    if (![x_min, x_max, y_min, y_max].every(Number.isFinite)) return;
    if (!(x_min < x_max) || !(y_min < y_max)) return;
    viewport = vp;
    backend.updateViewport(x_min, x_max, y_min, y_max);
    refresh();
  }

  function handleHover(px, py) {
    if (!Number.isFinite(px) || !Number.isFinite(py)) return;
    const res = backend.queryHover(px, py);
    hover = { px, py, math_x: res.math_x, math_y: res.math_y, marker: res.marker ?? null };
  }

  function handleToggle(key, visible) {
    toggles = { ...toggles, [key]: visible };
    if (!backend) return;
    // Canvas-only re-render, analysis retained (never setFunction).
    if (key === 'f' || key === 'fp' || key === 'fpp') {
      try {
        backend.setFunctionVisibility(key, !!visible);
      } catch {
        // Ignore, still refresh.
      }
    }
    refresh();
  }

  function pointValues() {
    if (!hasAnalyzed || !tangent) return { value: '—', slope: '—', behavior: '—', line: '—' };
    const y0 = tangent.y0;
    const slope = tangent.slope;
    if (!Number.isFinite(y0) || !Number.isFinite(slope)) return { value: '—', slope: '—', behavior: '—', line: '—' };
    const behavior = behaviorWord(slope);
    const line = `y = ${format3(y0)} + ${format3(slope)}·(x−${format3(x0)})`;
    return { value: format3(y0), slope: format3(slope), behavior, line };
  }

  function commitAnnounced() {
    const v = pointValues();
    announced = { x0, value: v.value, slope: v.slope, behavior: v.behavior };
  }

  async function handleShare() {
    if (!backend || shareBusy || !online || !hasAnalyzed) return;
    shareBusy = true;
    shareSlug = null;
    shareLink = '';
    shareNote = 'Saving…';
    shareTone = 'muted';
    shareRetry = false;
    await tick();
    const doc = backend.serializeWorkspace();
    const res = await saveWorkspace(doc);
    shareBusy = false;
    if (res.ok) {
      shareSlug = res.slug;
      shareLink = shortlink(res.slug);
      // First press creates + copies: confirm inline immediately instead of
      // showing Ready and requiring a second press.
      try {
        await navigator.clipboard.writeText(shareUrl(res.slug, window.location.origin));
        shareNote = 'Copied.';
        shareTone = 'success';
      } catch {
        shareNote = '';
        shareTone = 'muted';
        queueMicrotask(() => slugInputEl?.select?.());
        return;
      }
      queueMicrotask(() => slugInputEl?.select?.());
    } else if (res.error === 'too-large') {
      shareNote = 'Workspace too large to share.';
      shareTone = 'danger';
      shareRetry = true;
    } else if (res.error === 'network' || !online) {
      shareNote = "You're offline — sharing needs a connection.";
      shareTone = 'muted';
    } else {
      shareNote = "Couldn't create the link. Try again.";
      shareTone = 'danger';
      shareRetry = true;
    }
  }

  async function handleCopy() {
    if (!shareSlug) return;
    const full = shareUrl(shareSlug, window.location.origin);
    try {
      await navigator.clipboard.writeText(full);
    } catch {
      slugInputEl?.select?.();
      return;
    }
    shareNote = 'Copied.';
    shareTone = 'success';
  }

  async function restoreFromSlug(slug) {
    if (!backend || !isSlug(slug)) return;
    const loaded = await loadWorkspace(slug);
    if (!loaded.ok) {
      if (loaded.error === 'unknown') loadNote = 'Unknown workspace. Check the link and try again.';
      else if (loaded.error === 'network') loadNote = "You're offline. Showing your local workspace.";
      else loadNote = 'Could not load this workspace.';
      return;
    }
    const doc = loaded.doc;
    const res = restoreWorkspace(backend, doc);
    if (!res.ok) {
      loadNote = 'Could not load this workspace.';
      return;
    }
    if (typeof doc.functions?.f === 'string') {
      // Display the stored (normalized) expression; draft/committed align so
      // the badge shows fresh, not stale.
      committed = doc.functions.f;
      draft = doc.functions.f;
      try {
        const d = deriveAll(doc.functions.f);
        fPrimeDisplay = d.primeDisplay;
        fDoubleDisplay = d.doubleDisplay;
        // Re-install auto curves if the stored doc predates them.
        if (!doc.functions?.fp) {
          try {
            backend.setFunction('fp', d.primeBackend);
          } catch {}
        }
        if (!doc.functions?.fpp) {
          try {
            backend.setFunction('fpp', d.doubleBackend);
          } catch {}
        }
        try {
          backend.setFunctionStyle('f', DESIGN.f.color, DESIGN.f.width, DESIGN.f.dashed);
          backend.setFunctionStyle('fp', DESIGN.fp.color, DESIGN.fp.width, DESIGN.fp.dashed);
          backend.setFunctionStyle('fpp', DESIGN.fpp.color, DESIGN.fpp.width, DESIGN.fpp.dashed);
        } catch {}
      } catch {
        fPrimeDisplay = '';
        fDoubleDisplay = '';
      }
    }
    if (Number.isFinite(doc.params?.x0)) x0 = doc.params.x0;
    params = Object.fromEntries(Object.entries(doc.params ?? {}).filter(([k]) => k !== 'x0'));
    viewport = { ...doc.viewport };
    error = null;
    loadNote = '';
    hasAnalyzed = true;
    refresh();
    commitAnnounced();
  }

  onMount(async () => {
    backend = await getBackend();
    backend.setCanvasSize(800, 420);
    backend.updateParam('x0', x0);
    // Default domain −5…5 per UX; backend default may be −10…10.
    backend.updateViewport(DEFAULT_VIEW.x_min, DEFAULT_VIEW.x_max, DEFAULT_VIEW.y_min, DEFAULT_VIEW.y_max);
    viewport = { ...DEFAULT_VIEW };
    const slug = slugFromLocation(window.location);
    if (slug) {
      await restoreFromSlug(slug);
    } else {
      refresh();
    }
    const goOnline = () => (online = true);
    const goOffline = () => (online = false);
    window.addEventListener('online', goOnline);
    window.addEventListener('offline', goOffline);
  });

  const live = () => pointValues();
</script>

<main>
  <div class="hero">
    <p class="eyebrow">YOUR FRIENDLY CALCULUS TOOL</p>
    <h1>Make sense of <span class="accent">functions.</span></h1>
  </div>

  {#if backend}
    <p class="banner {backend.kind}" aria-live="off">
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

  <FormulaBar
    value={draft}
    {error}
    {analyzing}
    onInput={handleDraftInput}
    onAnalyze={() => handleAnalyze()}
    onClear={handleClear}
    onTry={handleTry}
  />

  <Sliders {params} onParam={handleParam} onCommit={handleParamCommit} />

  <div class="workspace">
    <div class="graph-col">
      <div class="place-toggles">
        <StyleControls {toggles} onToggle={handleToggle} />
      </div>
      <div class="place-canvas">
      <Canvas
        {curves}
        {viewport}
        {grid}
        {hover}
        {markers}
        {tangent}
        {asymptotes}
        showTangent={toggles.tangent !== false}
        showMarkers={toggles.markers !== false}
        {hasAnalyzed}
        onViewport={handleViewport}
        onHover={handleHover}
        onLeave={() => (hover = null)}
        onSize={(w, h) => backend?.setCanvasSize(w, h)}
        onFps={(value) => (fps = value)}
        onX0={handleX0}
      >
        <Tooltip {hover} />
      </Canvas>
      </div>
    </div>
    <div class="rail">
      <section class="card deriv-card" aria-label="First derivative">
        <div class="card-head">
          <span class="eyebrow">FIRST DERIVATIVE</span>
          {#if showBadge()}
            <span class="badge"><span class="dot" aria-hidden="true"></span>CALCULATED</span>
          {/if}
        </div>
        {#if hasAnalyzed && fPrimeDisplay}
          <p class="mono" aria-live="polite">f′(x) = {fPrimeDisplay}</p>
          <p class="body">The function's rate of change at every point.</p>
          {#if fDoubleDisplay}
            <p class="mono second">f″(x) = {fDoubleDisplay}</p>
          {/if}
        {:else}
          <p class="mono placeholder">f′(x) = —</p>
          <p class="body">Move along the curve</p>
        {/if}
      </section>

      <section class="card point-card" aria-label="Point analysis">
        <p class="eyebrow">POINT ANALYSIS</p>
        <div class="x-row">
          <label for="point-x">x</label>
          <input
            id="point-x"
            type="range"
            min={viewport.x_min}
            max={viewport.x_max}
            step="0.1"
            value={x0}
            aria-label="Point on curve"
            oninput={(e) => handleX0(parseFloat(e.target.value))}
            onchange={(e) => handleX0Commit(parseFloat(e.target.value))}
          />
          <span class="x-pill">{format3(x0)}</span>
        </div>
        <div class="values">
          <div class="value-row"><span>Function value</span><span class="mono-value">{live().value}</span></div>
          <div class="value-row"><span>Instantaneous slope</span><span class="mono-value">{live().slope}</span></div>
          <div class="value-row"><span>Local behavior</span><span class="behavior {live().behavior}">{live().behavior}</span></div>
          <div class="value-row"><span>Tangent line</span><span class="mono-value small">{live().line}</span></div>
        </div>
        <div class="sr-live" aria-live="polite">
          At x {format3(announced.x0)}, value {announced.value}, slope {announced.slope}, {announced.behavior}.
        </div>
      </section>

      {#if hasAnalyzed}
        <section class="share-row" aria-label="Share workspace">
          <label class="share-label" for="slug-field">Share</label>
          <div class="share-controls">
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
            {#if !shareSlug}
              <button
                class="copy-control"
                disabled={!backend || shareBusy || !online}
                title={online ? 'Create and copy share link' : "You're offline — sharing needs a connection."}
                onclick={handleShare}
              >{shareBusy ? 'Saving…' : 'Copy'}</button>
            {:else}
              <button class="copy-control" onclick={handleCopy}>Copy</button>
            {/if}
            {#if shareRetry}
              <button
                class="retry-control"
                disabled={shareBusy || !online}
                onclick={handleShare}
              >Retry</button>
            {/if}
          </div>
          <p class="status-note {shareTone}" aria-live="polite">
            {#if shareBusy}Saving…{:else if shareNote}{shareNote}{:else if shareSlug}Ready — copy the link.{:else if !online}You're offline — sharing needs a connection.{/if}
          </p>
        </section>
      {/if}
    </div>
  </div>

  {#if loadNote}
    <p class="status-note danger load-note" aria-live="polite">{loadNote}</p>
  {/if}

  {#if workspace}
    <details class="workspace-json">
      <summary>workspace JSON (version {workspace.version})</summary>
      <pre>{JSON.stringify(workspace, null, 2)}</pre>
    </details>
  {/if}
</main>

<style>
  main {
    max-width: 1120px;
  }
  .hero {
    margin: 16px 0;
  }
  .hero .eyebrow {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.12em;
    color: #6e6b8a;
    margin: 0 0 8px;
  }
  .hero h1 {
    font-size: 34px;
    font-weight: 800;
    line-height: 1.15;
    letter-spacing: -0.02em;
    color: #1e1b33;
    margin: 0;
  }
  .hero .accent {
    color: #4f46e5;
  }
  .workspace {
    display: grid;
    grid-template-columns: 2fr 1fr;
    column-gap: 24px;
    margin-top: 16px;
    align-items: start;
  }
  /* Independent columns: left stacks legend over canvas with a tight 8px
     gap, right stacks derivative over point over share with 16px rhythm.
     A shared grid row would force the short legend to the tall
     derivative card's height and reopen the gap below the toggles. */
  .graph-col {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .rail {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .place-toggles {
    min-width: 0;
  }
  .place-canvas {
    min-width: 0;
  }
  .card {
    background: #ffffff;
    border: 1px solid #e6e5f1;
    border-radius: 14px;
    padding: 16px;
  }
  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .eyebrow {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: #6e6b8a;
  }
  .badge {
    font-size: 12px;
    color: #16a34a;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .badge .dot {
    width: 8px;
    height: 8px;
    border-radius: 9999px;
    background: #16a34a;
  }
  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 15px;
    font-weight: 500;
  }
  .mono.placeholder {
    color: #9c99b8;
  }
  .mono.second {
    margin-top: 8px;
  }
  .body {
    font-size: 14px;
    color: #1e1b33;
  }
  .x-row {
    display: flex;
    gap: 12px;
    align-items: center;
    margin: 12px 0;
  }
  .x-row input[type='range'] {
    flex: 1;
    accent-color: #4f46e5;
  }
  .x-pill {
    background: #f1f0fa;
    border-radius: 9999px;
    padding: 4px 12px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 13px;
    font-weight: 600;
  }
  .values {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .value-row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    font-size: 14px;
    color: #6e6b8a;
  }
  .mono-value {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 13px;
    font-weight: 600;
    color: #1e1b33;
  }
  .mono-value.small {
    font-size: 12px;
  }
  .behavior.Increasing {
    color: #16a34a;
  }
  .behavior.Decreasing {
    color: #6e6b8a;
  }
  .behavior.Stationary {
    color: #e8930c;
  }
  .share-row {
    background: #f1f0fa;
    border-radius: 10px;
    padding: 12px;
  }
  .share-label {
    display: block;
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 4px;
  }
  .share-controls {
    display: flex;
    gap: 8px;
  }
  .slug-field {
    flex: 1;
    min-width: 0;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 14px;
    font-weight: 600;
    border: 1px solid #e6e5f1;
    border-radius: 6px;
    padding: 8px;
    min-height: 44px;
  }
  .copy-control,
  .retry-control {
    border: 1px solid #e6e5f1;
    background: #fff;
    border-radius: 6px;
    padding: 8px 12px;
    min-height: 44px;
    cursor: pointer;
  }
  .sr-live {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
  }
  @media (prefers-reduced-motion: reduce) {
    * {
      transition: none !important;
    }
  }
</style>
