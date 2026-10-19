<script>
  // Per-curve style controls (4.3, E4-STYLE-STRUCT): one instance per demo
  // curve, wired by App to `setFunctionStyle` / `setFunctionVisibility`.
  // Same pattern as Sliders.svelte: validate here, App guards again.
  let { curveId = 'f', colorHex, widthPx, dashed, visible, onStyle, onDashed, onVisibility } = $props();

  const colorId = `style-color-${curveId}`;
  const dashedId = `style-dashed-${curveId}`;
  const visibleId = `style-visible-${curveId}`;

  function forwardStyle(hex) {
    if (typeof hex !== 'string' || !/^#[0-9a-fA-F]{6}$/.test(hex)) return;
    onStyle(hex);
  }
</script>

<div class="style-controls">
  <span class="curve-id" title="function id">{curveId}</span>
  <label for={colorId}>curve colour</label>
  <input
    id={colorId}
    type="color"
    value={colorHex}
    aria-label="curve colour {curveId}"
    title="curve colour {colorHex}"
    oninput={(e) => forwardStyle(e.target.value)}
  />
  <span class="width-label" title="ribbon width in px">{Number(widthPx ?? 3.5).toFixed(1)} px</span>
  <label for={dashedId}>
    <input
      id={dashedId}
      type="checkbox"
      checked={dashed}
      aria-label="dashed style {curveId}"
      title="dashed style (derivative overlay demo)"
      onchange={(e) => onDashed(e.target.checked)}
    />
    dashed
  </label>
  <label for={visibleId}>
    <input
      id={visibleId}
      type="checkbox"
      checked={visible}
      aria-label="curve visible {curveId}"
      onchange={(e) => onVisibility(e.target.checked)}
    />
    visible
  </label>
</div>
