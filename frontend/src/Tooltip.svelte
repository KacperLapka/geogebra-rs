<script>
  // Hover state is owned by Rust: { px, py, math_x, math_y, marker } comes
  // verbatim from query_hover. This component formats text only and never
  // performs spatial math (AD-4).
  let { hover } = $props();

  const fmt = (v) => (v === null || v === undefined || Number.isNaN(v) ? '—' : Number(v).toFixed(3));
</script>

{#if hover}
  <div class="tooltip" style="left: {hover.px + 14}px; top: {hover.py + 14}px;">
    ({fmt(hover.math_x)}, {fmt(hover.math_y)}){#if hover.marker}{` · ${hover.marker}`}{/if}
  </div>
{/if}
