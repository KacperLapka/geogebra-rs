<script>
  // Hover state owned by Rust: { px, py, math_x, math_y, marker } verbatim
  // from query_hover. Text formatting only, never spatial math (AD-4).
  // Non-finite reads as an em-dash, never NaN/Infinity as finite.
  let { hover } = $props();

  const fmt = (v) => (v === null || v === undefined || !Number.isFinite(v) ? '—' : Number(v).toFixed(3));
</script>

{#if hover}
  <div class="tooltip" style="left: {hover.px + 14}px; top: {hover.py + 14}px;" aria-hidden="true">
    ({fmt(hover.math_x)}, {fmt(hover.math_y)}){#if hover.marker}{` · ${hover.marker}`}{/if}
  </div>
{/if}

<style>
  .tooltip {
    position: absolute;
    pointer-events: none;
    background: rgba(30, 27, 51, 0.92);
    color: #fff;
    font-size: 12px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    padding: 4px 8px;
    border-radius: 6px;
    white-space: nowrap;
    z-index: 5;
  }
</style>
