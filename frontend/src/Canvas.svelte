<script>
  let { points, viewport, grid, hover, onViewport, onHover, onLeave, onSize, onFps, children } = $props();

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

    // grid
    ctx.strokeStyle = '#8883';
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (const t of grid.x ?? []) {
      const [px] = toScreen(t, 0, w, h);
      ctx.moveTo(px, 0);
      ctx.lineTo(px, h);
    }
    for (const t of grid.y ?? []) {
      const [, py] = toScreen(0, t, w, h);
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
      ctx.moveTo(0, zeroY);
      ctx.lineTo(w, zeroY);
      ctx.moveTo(zeroX, 0);
      ctx.lineTo(zeroX, h);
    }
    ctx.stroke();

    // curve (breaks on null y)
    ctx.strokeStyle = '#2563eb';
    ctx.lineWidth = 2;
    ctx.beginPath();
    let pen = false;
    for (const [x, y] of points ?? []) {
      if (y === null || y === undefined || Number.isNaN(y)) {
        pen = false;
        continue;
      }
      const [px, py] = toScreen(x, y, w, h);
      if (!pen) {
        ctx.moveTo(px, py);
        pen = true;
      } else {
        ctx.lineTo(px, py);
      }
    }
    ctx.stroke();

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
    void points;
    void viewport;
    void grid;
    void hover;
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
  {@render children?.()}
</div>
