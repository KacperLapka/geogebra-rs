<script>
  // Multi-curve canvas (4.3): `curves` is a list of `{id, points, style}`
  // with per-curve `{color, widthPx, dashed, visible}`. Legacy single-curve
  // `points`/`style` props still work: they render as one curve.
  // Calculus overlays (2.6): `markers` is `[{kind, x, y}]` and `tangent` is
  // `{from, to}` in math coords, stroked verbatim from Rust (AD-4).
  // Vertical asymptotes (3.1): `asymptotes` is `[{x, from, to}]` in math
  // coords, stroked dashed verbatim from Rust.
  let { curves, points, viewport, grid, hover, style, markers, tangent, asymptotes, onViewport, onHover, onLeave, onSize, onFps, children } = $props();

  let canvas = $state(null);
  let ctx = $state(null);
  let dragging = $state(false);
  let last = $state(null);
  // Viewport loop: data effects mark the frame dirty, the rAF loop below
  // presents it. Pan/zoom streams viewport updates; the loop draws at most
  // once per frame, so continuous drags never queue blank frames.
  let dirty = $state(true);

  const toScreen = (x, y, w, h) => [
    ((x - viewport.x_min) / (viewport.x_max - viewport.x_min)) * w,
    ((viewport.y_max - y) / (viewport.y_max - viewport.y_min)) * h
  ];

  // DOM label positioning only: fraction along each axis via the same
  // `toScreen` mapping above (no tick math here; Rust owns ticks). Labels
  // render in the overlay below and may lag the canvas stroke by one frame
  // during continuous drag, per E4-VIEWPORT-GRID.
  const xFrac = (v) => (v - viewport.x_min) / (viewport.x_max - viewport.x_min);
  const yFrac = (v) => (viewport.y_max - v) / (viewport.y_max - viewport.y_min);
  const xSpanOk = () =>
    Number.isFinite(viewport.x_max - viewport.x_min) && viewport.x_max > viewport.x_min;
  const ySpanOk = () =>
    Number.isFinite(viewport.y_max - viewport.y_min) && viewport.y_max > viewport.y_min;

  function fitCanvas() {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.round(rect.width * dpr));
    canvas.height = Math.max(1, Math.round(rect.height * dpr));
    onSize(rect.width, rect.height);
    dirty = true;
  }

  function draw() {
    if (!canvas || !ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.width / dpr;
    const h = canvas.height / dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    // grid: Rust-owned majors sync plus faint minors, same frame.
    // Majors use the established tone; minors subdivide strictly between
    // majors (÷5, or ÷4 for 2-mantissa steps) at a fainter alpha.
    ctx.strokeStyle = '#8883';
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (const t of grid.x ?? []) {
      if (!Number.isFinite(t)) continue;
      const [px] = toScreen(t, 0, w, h);
      if (!Number.isFinite(px)) continue;
      ctx.moveTo(px, 0);
      ctx.lineTo(px, h);
    }
    for (const t of grid.y ?? []) {
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
    for (const t of grid.x_minor ?? []) {
      if (!Number.isFinite(t)) continue;
      const [px] = toScreen(t, 0, w, h);
      if (!Number.isFinite(px)) continue;
      ctx.moveTo(px, 0);
      ctx.lineTo(px, h);
    }
    for (const t of grid.y_minor ?? []) {
      if (!Number.isFinite(t)) continue;
      const [, py] = toScreen(0, t, w, h);
      if (!Number.isFinite(py)) continue;
      ctx.moveTo(0, py);
      ctx.lineTo(w, py);
    }
    ctx.stroke();

    // axes
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

    // curves (breaks on null y; skipped when the style hides it)
    // One draw block per curve: colour, clamped width, dash + offset reset,
    // visibility skip. Viewport/hover math untouched.
    const list = curves ?? (points ? [{ id: 'f', points, style }] : []);
    for (const curve of list) {
      const st = curve.style ?? style;
      if (st?.visible === false) continue;
      ctx.strokeStyle = st?.color ?? '#2563eb';
      // Clamp like every other layer: ABI + shader enforce [2.5, 6.0].
      ctx.lineWidth = Number.isFinite(st?.widthPx) ? Math.min(6.0, Math.max(2.5, st.widthPx)) : 3.5;
      // 2D parity with the GPU dash: 8px-on / 6px-off screen-space, solid resets.
      ctx.setLineDash(st?.dashed ? [8, 6] : []);
      ctx.lineDashOffset = 0;
      ctx.beginPath();
      let pen = false;
      for (const [x, y] of curve.points ?? []) {
        if (y === null || y === undefined || Number.isNaN(y)) {
          pen = false;
          continue;
        }
        const [px, py] = toScreen(x, y, w, h);
        if (!pen) {
          // GPU restarts arclength at each NaN break: restart 2D dash phase too.
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

    // tangent segment (2.6): Rust-computed math coords stroked verbatim.
    if (tangent && Array.isArray(tangent.from) && Array.isArray(tangent.to)) {
      const [ax, ay] = toScreen(tangent.from[0], tangent.from[1], w, h);
      const [bx, by] = toScreen(tangent.to[0], tangent.to[1], w, h);
      if ([ax, ay, bx, by].every(Number.isFinite)) {
        ctx.strokeStyle = '#ea580c';
        ctx.lineWidth = 2;
        ctx.setLineDash([]);
        ctx.lineDashOffset = 0;
        ctx.beginPath();
        ctx.moveTo(ax, ay);
        ctx.lineTo(bx, by);
        ctx.stroke();
      }
    }

    // vertical asymptotes (3.1): Rust-computed x with viewport-spanning
    // endpoints stroked verbatim, dashed [8,6] like the curve dash parity.
    for (const a of asymptotes ?? []) {
      if (!a || !Array.isArray(a.from) || !Array.isArray(a.to)) continue;
      if (!Number.isFinite(a.x)) continue;
      const [ax, ay] = toScreen(a.from[0], a.from[1], w, h);
      const [bx, by] = toScreen(a.to[0], a.to[1], w, h);
      if (![ax, ay, bx, by].every(Number.isFinite)) continue;
      ctx.strokeStyle = '#6b7280';
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

    // marker circles (2.6): per-kind fill, white ring for contrast.
    const MARKER_COLORS = { root: '#dc2626', min: '#16a34a', max: '#d97706', inflection: '#9333ea' };
    for (const m of markers ?? []) {
      if (!m || !Number.isFinite(m.x) || !Number.isFinite(m.y)) continue;
      const [mx, my] = toScreen(m.x, m.y, w, h);
      if (!Number.isFinite(mx) || !Number.isFinite(my)) continue;
      ctx.fillStyle = MARKER_COLORS[m.kind] ?? '#dc2626';
      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.arc(mx, my, 5, 0, 2 * Math.PI);
      ctx.fill();
      ctx.stroke();
    }

    // hover marker
    if (hover && hover.math_y !== null && hover.math_y !== undefined) {
      const [hx, hy] = toScreen(hover.math_x, hover.math_y, w, h);
      ctx.fillStyle = '#dc2626';
      ctx.beginPath();
      ctx.arc(hx, hy, 4, 0, 2 * Math.PI);
      ctx.fill();
    }
  }

  function pos(e) {
    const rect = canvas.getBoundingClientRect();
    return [e.clientX - rect.left, e.clientY - rect.top, rect.width, rect.height];
  }

  function wheel(e) {
    e.preventDefault();
    const [px, py, w, h] = pos(e);
    const fx = px / w;
    const fy = py / h;
    const k = e.deltaY > 0 ? 1.15 : 1 / 1.15;
    const xw = viewport.x_max - viewport.x_min;
    const yw = viewport.y_max - viewport.y_min;
    const nxw = Math.min(1e6, Math.max(1e-6, xw * k));
    const nyw = Math.min(1e6, Math.max(1e-6, yw * k));
    const cx = viewport.x_min + fx * xw;
    const cy = viewport.y_max - fy * yw;
    onViewport({
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
    onHover(px, py);
    if (!dragging) return;
    const [lx, ly] = last;
    const dx = ((lx - px) / w) * (viewport.x_max - viewport.x_min);
    const dy = ((py - ly) / h) * (viewport.y_max - viewport.y_min);
    last = [px, py, w, h];
    onViewport({
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

  $effect(() => {
    if (canvas && !ctx) ctx = canvas.getContext('2d');
    fitCanvas();

    // Canvas event loop: present dirty frames, meter the 60 FPS baseline.
    // FPS is reported at most once per second to keep parent state calm.
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
    // mark dirty on data changes; the loop presents
    void curves;
    void points;
    void viewport;
    void grid;
    void hover;
    void style;
    void markers;
    void tangent;
    void asymptotes;
    dirty = true;
  });
</script>

<svelte:window onresize={fitCanvas} />

<div class="canvas-wrap">
  <canvas
    bind:this={canvas}
    onwheel={wheel}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointerleave={() => {
      up();
      onLeave();
    }}
  ></canvas>
  <!-- Tick labels: transparent DOM overlay only (never canvas text/atlas).
       Positioned from bridge `{value, text}` via the `toScreen` fractions
       above; geometry strokes sync while these may lag one frame. -->
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
  {@render children?.()}
</div>

<style>
  .canvas-wrap {
    position: relative;
  }
  .canvas-wrap canvas {
    display: block;
    width: 100%;
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
    color: #6b7280;
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
</style>
