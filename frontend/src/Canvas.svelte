<script>
  // Graph canvas: 2D dirty-rAF loop presenting Rust-owned geometry.
  // Curves + grid + tangent + asymptotes stroke on canvas; analysis markers
  // are DOM buttons positioned via the same toScreen mapping (xFrac/yFrac
  // percentages) updated in the same frame as curves — never layout-thrash,
  // 60 FPS under 16ms. Svelte positions tooltips only, never spatial math.
  // Fixed DESIGN palette: f indigo solid, f' green dashed, f'' purple
  // dashed, tangent amber dashed, asymptotes gray dashed.
  import { onMount } from 'svelte';

  let {
    curves,
    viewport,
    grid,
    hover,
    markers,
    tangent,
    asymptotes,
    showTangent = true,
    showMarkers = true,
    hasAnalyzed = false,
    onViewport,
    onHover,
    onLeave,
    onSize,
    onFps,
    onX0,
    children
  } = $props();

  let canvas = $state(null);
  let ctx = $state(null);
  let dragging = $state(false);
  let last = $state(null);
  let dirty = $state(true);
  let webglFail = $state(false);

  const DEFAULT_VIEW = { x_min: -5, x_max: 5, y_min: -5, y_max: 5 };

  const toScreen = (x, y, w, h) => [
    ((x - viewport.x_min) / (viewport.x_max - viewport.x_min)) * w,
    ((viewport.y_max - y) / (viewport.y_max - viewport.y_min)) * h
  ];

  // DOM overlay positioning only (percentages, same mapping as toScreen).
  const xFrac = (v) => (v - viewport.x_min) / (viewport.x_max - viewport.x_min);
  const yFrac = (v) => (viewport.y_max - v) / (viewport.y_max - viewport.y_min);
  const xSpanOk = () =>
    Number.isFinite(viewport.x_max - viewport.x_min) && viewport.x_max > viewport.x_min;
  const ySpanOk = () =>
    Number.isFinite(viewport.y_max - viewport.y_min) && viewport.y_max > viewport.y_min;

  // Single visibility truth for the DOM marker layer: finite coords, sane
  // spans, and fractions inside [0, 1]. Out-of-range means skip render
  // (hidden), never a clamped/floating dot mid-canvas. Uses the same
  // xFrac/yFrac fractions that position the button, which share toScreen's
  // mapping, so canvas strokes and DOM dots agree in every viewport.
  const markerOnScreen = (m) => {
    if (!Number.isFinite(m?.x) || !Number.isFinite(m?.y)) return false;
    if (!xSpanOk() || !ySpanOk()) return false;
    const fx = xFrac(m.x);
    const fy = yFrac(m.y);
    return fx >= 0 && fx <= 1 && fy >= 0 && fy <= 1;
  };

  const MARKER_STYLE = {
    root: { color: '#dc2626', label: 'Root' },
    min: { color: '#4f46e5', label: 'Minimum' },
    max: { color: '#4f46e5', label: 'Maximum' },
    inflection: { color: '#1fa36b', label: 'Inflection' }
  };
  const markerMeta = (m) => MARKER_STYLE[m?.kind] ?? { color: '#4f46e5', label: m?.kind ?? 'Marker' };
  const markerLabel = (m) => {
    const meta = markerMeta(m);
    const xs = Number.isFinite(m.x) ? Number(m.x).toFixed(2) : '—';
    return `${meta.label} at x = ${xs}`;
  };
  const markerTip = (m) => {
    const f = (v) => (Number.isFinite(v) ? Number(v).toFixed(3) : '—');
    return `(${f(m.x)}, ${f(m.y)})`;
  };

  function fitCanvas() {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.round(rect.width * dpr));
    canvas.height = Math.max(1, Math.round(rect.height * dpr));
    onSize?.(rect.width, rect.height);
    dirty = true;
  }

  function draw() {
    if (!canvas || !ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.width / dpr;
    const h = canvas.height / dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    // Grid: Rust-owned majors + faint minors, same frame.
    ctx.strokeStyle = '#8883';
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (const t of grid?.x ?? []) {
      if (!Number.isFinite(t)) continue;
      const [px] = toScreen(t, 0, w, h);
      if (!Number.isFinite(px)) continue;
      ctx.moveTo(px, 0);
      ctx.lineTo(px, h);
    }
    for (const t of grid?.y ?? []) {
      if (!Number.isFinite(t)) continue;
      const [, py] = toScreen(0, t, w, h);
      if (!Number.isFinite(py)) continue;
      ctx.moveTo(0, py);
      ctx.lineTo(w, py);
    }
    ctx.stroke();
    ctx.strokeStyle = '#8882';
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (const t of grid?.x_minor ?? []) {
      if (!Number.isFinite(t)) continue;
      const [px] = toScreen(t, 0, w, h);
      if (!Number.isFinite(px)) continue;
      ctx.moveTo(px, 0);
      ctx.lineTo(px, h);
    }
    for (const t of grid?.y_minor ?? []) {
      if (!Number.isFinite(t)) continue;
      const [, py] = toScreen(0, t, w, h);
      if (!Number.isFinite(py)) continue;
      ctx.moveTo(0, py);
      ctx.lineTo(w, py);
    }
    ctx.stroke();

    // Axes.
    ctx.strokeStyle = '#888a';
    ctx.beginPath();
    {
      const [, zeroY] = toScreen(0, 0, w, h);
      const [zeroX] = toScreen(0, 0, w, h);
      if (Number.isFinite(zeroX) && Number.isFinite(zeroY)) {
        ctx.moveTo(0, zeroY);
        ctx.lineTo(w, zeroY);
        ctx.moveTo(zeroX, 0);
        ctx.lineTo(zeroX, h);
      }
    }
    ctx.stroke();

    // Curves: fixed DESIGN palette wins; bridge styles already set to it on
    // Analyze, fallback here keeps the canvas honest if a style lags.
    const DESIGN_FALLBACK = {
      f: { color: '#4f46e5', dashed: false },
      fp: { color: '#1fa36b', dashed: true },
      fpp: { color: '#9333ea', dashed: true }
    };
    const list = curves ?? [];
    for (const curve of list) {
      const st = curve.style ?? {};
      if (st.visible === false) continue;
      const fb = DESIGN_FALLBACK[curve.id];
      ctx.strokeStyle = st.color ?? fb?.color ?? '#4f46e5';
      ctx.lineWidth = Number.isFinite(st.widthPx) ? Math.min(6.0, Math.max(2.5, st.widthPx)) : 2.5;
      const dashed = st.dashed ?? fb?.dashed ?? curve.id !== 'f';
      ctx.setLineDash(dashed ? [8, 6] : []);
      ctx.lineDashOffset = 0;
      ctx.beginPath();
      let pen = false;
      for (const [x, y] of curve.points ?? []) {
        if (y === null || y === undefined || Number.isNaN(y)) {
          pen = false;
          continue;
        }
        const [px, py] = toScreen(x, y, w, h);
        if (!Number.isFinite(px) || !Number.isFinite(py)) {
          pen = false;
          continue;
        }
        if (!pen) {
          ctx.lineDashOffset = 0;
          ctx.moveTo(px, py);
          pen = true;
        } else {
          ctx.lineTo(px, py);
        }
      }
      ctx.stroke();
    }
    ctx.setLineDash([]);
    ctx.lineDashOffset = 0;

    // Tangent (amber dashed) — canvas-only toggle, never refires analysis.
    if (showTangent && tangent && Array.isArray(tangent.from) && Array.isArray(tangent.to)) {
      const [ax, ay] = toScreen(tangent.from[0], tangent.from[1], w, h);
      const [bx, by] = toScreen(tangent.to[0], tangent.to[1], w, h);
      if ([ax, ay, bx, by].every(Number.isFinite)) {
        ctx.strokeStyle = '#e8930c';
        ctx.lineWidth = 2;
        ctx.setLineDash([8, 6]);
        ctx.lineDashOffset = 0;
        ctx.beginPath();
        ctx.moveTo(ax, ay);
        ctx.lineTo(bx, by);
        ctx.stroke();
        ctx.setLineDash([]);
      }
    }

    // Asymptotes (gray dashed, never bridged — NaN breaks above).
    for (const a of asymptotes ?? []) {
      if (!a || !Array.isArray(a.from) || !Array.isArray(a.to)) continue;
      if (!Number.isFinite(a.x)) continue;
      const [ax, ay] = toScreen(a.from[0], a.from[1], w, h);
      const [bx, by] = toScreen(a.to[0], a.to[1], w, h);
      if (![ax, ay, bx, by].every(Number.isFinite)) continue;
      ctx.strokeStyle = '#b9b8d0';
      ctx.lineWidth = 1.5;
      ctx.setLineDash([8, 6]);
      ctx.lineDashOffset = 0;
      ctx.beginPath();
      ctx.moveTo(ax, ay);
      ctx.lineTo(bx, by);
      ctx.stroke();
    }
    ctx.setLineDash([]);
    ctx.lineDashOffset = 0;

    // Hover marker (2D dot, not a keyboard marker).
    if (hover && hover.math_y !== null && hover.math_y !== undefined && Number.isFinite(hover.math_y)) {
      const [hx, hy] = toScreen(hover.math_x, hover.math_y, w, h);
      if (Number.isFinite(hx) && Number.isFinite(hy)) {
        ctx.fillStyle = '#dc2626';
        ctx.beginPath();
        ctx.arc(hx, hy, 4, 0, 2 * Math.PI);
        ctx.fill();
      }
    }
  }

  function pos(e) {
    const rect = canvas.getBoundingClientRect();
    return [e.clientX - rect.left, e.clientY - rect.top, rect.width, rect.height];
  }

  function zoomAt(factor, cxFrac = 0.5, cyFrac = 0.5) {
    const xw = viewport.x_max - viewport.x_min;
    const yw = viewport.y_max - viewport.y_min;
    const nxw = Math.min(1e6, Math.max(1e-6, xw * factor));
    const nyw = Math.min(1e6, Math.max(1e-6, yw * factor));
    const cx = viewport.x_min + cxFrac * xw;
    const cy = viewport.y_max - cyFrac * yw;
    onViewport?.({
      x_min: cx - cxFrac * nxw,
      x_max: cx + (1 - cxFrac) * nxw,
      y_min: cy - (1 - cyFrac) * nyw,
      y_max: cy + cyFrac * nyw
    });
  }

  function wheel(e) {
    e.preventDefault();
    const [px, py, w, h] = pos(e);
    const fx = w ? px / w : 0.5;
    const fy = h ? py / h : 0.5;
    const k = e.deltaY > 0 ? 1.15 : 1 / 1.15;
    const xw = viewport.x_max - viewport.x_min;
    const yw = viewport.y_max - viewport.y_min;
    const nxw = Math.min(1e6, Math.max(1e-6, xw * k));
    const nyw = Math.min(1e6, Math.max(1e-6, yw * k));
    const cx = viewport.x_min + fx * xw;
    const cy = viewport.y_max - fy * yw;
    onViewport?.({
      x_min: cx - fx * nxw,
      x_max: cx + (1 - fx) * nxw,
      y_min: cy - (1 - fy) * nyw,
      y_max: cy + fy * nyw
    });
  }

  function down(e) {
    dragging = true;
    last = pos(e);
    canvas.setPointerCapture(e.pointerId);
  }

  function move(e) {
    const [px, py, w, h] = pos(e);
    onHover?.(px, py);
    if (!dragging) return;
    const [lx, ly] = last;
    const dx = ((lx - px) / w) * (viewport.x_max - viewport.x_min);
    const dy = ((py - ly) / h) * (viewport.y_max - viewport.y_min);
    last = [px, py, w, h];
    onViewport?.({
      x_min: viewport.x_min + dx,
      x_max: viewport.x_max + dx,
      y_min: viewport.y_min + dy,
      y_max: viewport.y_max + dy
    });
  }

  function up() {
    dragging = false;
    last = null;
  }

  function fmtDomain(v) {
    if (!Number.isFinite(v)) return '—';
    const s = Math.abs(v) < 1e-9 ? '0' : String(Math.round(v * 100) / 100);
    return s.replace('-', '−');
  }

  const hasPoints = $derived(
    curves?.some((c) => (c.points ?? []).some((p) => p[1] !== null && p[1] !== undefined)) ?? false
  );
  const allCurvesHidden = $derived(
    !!(curves?.length && !(curves?.some((c) => c.style?.visible !== false)))
  );
  const allOff = $derived(!!(hasAnalyzed && allCurvesHidden && !showTangent && !showMarkers));

  $effect(() => {
    if (canvas && !ctx) ctx = canvas.getContext('2d');
    fitCanvas();
    let raf = 0;
    let frames = 0;
    let windowStart = performance.now();
    let lastReport = 0;
    const loop = (now) => {
      if (dirty) {
        draw();
        dirty = false;
      }
      frames += 1;
      const elapsed = now - windowStart;
      if (elapsed >= 500) {
        const value = Math.round((frames * 1000) / elapsed);
        if (now - lastReport >= 1000) {
          lastReport = now;
          onFps?.(value);
        }
        frames = 0;
        windowStart = now;
      }
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  });

  $effect(() => {
    void curves;
    void viewport;
    void grid;
    void hover;
    void markers;
    void tangent;
    void asymptotes;
    void showTangent;
    void showMarkers;
    dirty = true;
  });

  onMount(async () => {
    // Double-failure fallback only: WebGPU first, WebGL2 retry; the message
    // shows solely when both fail, workspace stays usable on 2D.
    try {
      let ok = false;
      if (typeof navigator !== 'undefined' && navigator.gpu?.requestAdapter) {
        try {
          const a = await navigator.gpu.requestAdapter();
          if (a) ok = true;
        } catch {
          ok = false;
        }
      }
      if (!ok && canvas) {
        try {
          const gl = document.createElement('canvas').getContext('webgl2');
          if (gl) ok = true;
        } catch {
          ok = false;
        }
      }
      webglFail = !ok;
    } catch {
      webglFail = true;
    }
  });
</script>

<svelte:window onresize={fitCanvas} />

<div class="graph-card">
  <div class="toolbar">
    <span class="domain" aria-live="off">Showing x from {fmtDomain(viewport.x_min)} to {fmtDomain(viewport.x_max)}</span>
    <span class="spacer"></span>
    <button type="button" class="tool" aria-label="Zoom in" title="Zoom in" onclick={() => zoomAt(1 / 1.25)}>+</button>
    <button type="button" class="tool" aria-label="Zoom out" title="Zoom out" onclick={() => zoomAt(1.25)}>−</button>
    <button type="button" class="tool" aria-label="Reset view" title="Reset view" onclick={() => onViewport?.({ ...DEFAULT_VIEW })}>Reset</button>
  </div>
  <div class="canvas-wrap">
    <!-- svelte-ignore a11y_no_interactive_element_to_noninteractive_role -->
    <canvas
      bind:this={canvas}
      role="img"
      aria-label={markers?.length ? `Graph with ${markers.length} analysis markers` : 'Graph of the analyzed function'}
      onwheel={wheel}
      onpointerdown={down}
      onpointermove={move}
      onpointerup={up}
      onpointerleave={() => {
        up();
        onLeave?.();
      }}
    ></canvas>
    <div class="grid-overlay" aria-hidden="true">
      {#if xSpanOk()}
        {#each grid?.x_labels ?? [] as l (l.value)}
          {#if Number.isFinite(l?.value) && typeof l?.text === 'string'}
            <span class="grid-label x" style="left: {xFrac(l.value) * 100}%">{l.text}</span>
          {/if}
        {/each}
      {/if}
      {#if ySpanOk()}
        {#each grid?.y_labels ?? [] as l (l.value)}
          {#if Number.isFinite(l?.value) && typeof l?.text === 'string'}
            <span class="grid-label y" style="top: {yFrac(l.value) * 100}%">{l.text}</span>
          {/if}
        {/each}
      {/if}
    </div>
    {#if showMarkers}
      <div class="marker-layer">
        {#each (markers ?? []) as m (m.kind + ':' + m.x + ':' + m.y)}
          {#if markerOnScreen(m)}
            {@const meta = markerMeta(m)}
            <button
              type="button"
              class="marker"
              style="left:{xFrac(m.x) * 100}%;top:{yFrac(m.y) * 100}%;--marker-color:{meta.color}"
              aria-label={markerLabel(m)}
              title={markerTip(m)}
              onclick={() => {
                if (Number.isFinite(m.x)) onX0?.(m.x);
              }}
            >
              <span class="dot" aria-hidden="true"></span>
              <span class="tip" aria-hidden="true">{markerTip(m)}</span>
            </button>
          {/if}
        {/each}
      </div>
    {/if}
    {@render children?.()}
    {#if !allOff && !hasPoints}
      <div class="empty-hint">Type a function and hit Analyze.</div>
    {/if}
    {#if allOff}
      <div class="empty-hint">All layers off — toggling any curve back on restores the canvas.</div>
    {/if}
  </div>
  {#if webglFail}
    <p class="fallback-note" role="note">Your browser can't draw the graph. Try a recent Chrome, Edge, or Firefox.</p>
  {/if}
  <p class="sr-only">Graph canvas fallback: use the point analysis values for exact coordinates.</p>
</div>

<style>
  .graph-card {
    background: #ffffff;
    border: 1px solid #e6e5f1;
    border-radius: 14px;
    padding: 12px;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }
  .domain {
    font-size: 12px;
    color: #6e6b8a;
  }
  .spacer {
    flex: 1;
  }
  .tool {
    border: 1px solid #e6e5f1;
    background: #fff;
    border-radius: 6px;
    min-width: 44px;
    min-height: 44px;
    font-size: 16px;
    cursor: pointer;
    color: #1e1b33;
  }
  .tool:focus-visible {
    outline: 2px solid #4f46e5;
    outline-offset: 2px;
  }
  .canvas-wrap {
    position: relative;
    touch-action: none;
  }
  .canvas-wrap canvas {
    display: block;
    width: 100%;
    height: 420px;
    cursor: crosshair;
  }
  .grid-overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }
  .grid-label {
    position: absolute;
    font-size: 10px;
    line-height: 1;
    color: #9c99b8;
    background: transparent;
    white-space: nowrap;
    pointer-events: none;
  }
  .grid-label.x {
    bottom: 2px;
    transform: translateX(-50%);
  }
  .grid-label.y {
    left: 4px;
    transform: translateY(-50%);
  }
  .marker-layer {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .marker {
    position: absolute;
    transform: translate(-50%, -50%);
    background: transparent;
    border: none;
    padding: 0;
    margin: 0;
    width: 44px;
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    pointer-events: auto;
  }
  .marker:focus-visible {
    outline: 2px solid #4f46e5;
    outline-offset: 2px;
    border-radius: 9999px;
  }
  .marker .dot {
    display: block;
    flex: none;
    width: 12px;
    height: 12px;
    border-radius: 9999px;
    background: var(--marker-color);
    border: 2px solid #fff;
    box-shadow: 0 0 0 1px var(--marker-color);
    margin: auto;
  }
  .marker .tip {
    display: none;
    position: absolute;
    left: 50%;
    top: -8px;
    transform: translate(-50%, -100%);
    background: rgba(30, 27, 51, 0.92);
    color: #fff;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
    padding: 4px 8px;
    border-radius: 6px;
    white-space: nowrap;
  }
  .marker:hover .tip,
  .marker:focus-visible .tip {
    display: block;
  }
  .empty-hint {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #9c99b8;
    font-size: 14px;
    pointer-events: none;
  }
  .fallback-note {
    color: #6e6b8a;
    font-size: 13px;
    margin: 8px 0 0;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
  }
  @media (prefers-reduced-motion: reduce) {
    .marker,
    .tool {
      transition: none;
    }
  }
</style>
