<script>
  let { hover } = $props();

  const fmt = (v) => (v === null || v === undefined || Number.isNaN(v) ? '—' : Number(v).toFixed(3));
</script>

{#if hover}
  <div class="tooltip" style="left: {hover.px + 14}px; top: {hover.py + 14}px;">
    ({fmt(hover.math_x)}, {fmt(hover.math_y)})
  </div>
{/if}
