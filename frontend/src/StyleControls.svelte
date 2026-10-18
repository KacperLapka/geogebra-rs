<script>
  // Thin style controls for the single tracer curve `f` (E4-STYLE-STRUCT).
  // Forwards to `setFunctionStyle` / `setFunctionVisibility`; same pattern
  // as Sliders.svelte: validate here, App guards again. No palette UI and
  // no dash toggle — those land in entries 4.2/4.3.
  let { colorHex, widthPx, visible, onStyle, onVisibility } = $props();

  function forwardStyle(hex) {
    if (typeof hex !== 'string' || !/^#[0-9a-fA-F]{6}$/.test(hex)) return;
    onStyle(hex);
  }
</script>

<div class="style-controls">
  <label for="style-color">curve colour</label>
  <input
    id="style-color"
    type="color"
    value={colorHex}
    aria-label="curve colour"
    title="curve colour {colorHex}"
    oninput={(e) => forwardStyle(e.target.value)}
  />
  <span class="width-label" title="ribbon width in px">{Number(widthPx ?? 3.5).toFixed(1)} px</span>
  <label for="style-visible">
    <input
      id="style-visible"
      type="checkbox"
      checked={visible}
      aria-label="curve visible"
      onchange={(e) => onVisibility(e.target.checked)}
    />
    visible
  </label>
</div>
