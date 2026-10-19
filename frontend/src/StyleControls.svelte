<script>
  // R10 fixed palette (DESIGN wins over fallback blue): legend swatches
  // mirror curve style exactly; toggles re-render canvas only, never refire
  // analysis. No free color picker in this epic. Single primary + auto
  // f/f'/f''/tangent only.
  let { toggles, onToggle } = $props();

  const LAYERS = [
    { key: 'f', label: 'f(x)', swatch: 'solid-indigo' },
    { key: 'fp', label: "f′(x)", swatch: 'dashed-green' },
    { key: 'fpp', label: "f″(x)", swatch: 'dashed-purple' },
    { key: 'tangent', label: 'tangent', swatch: 'dashed-amber' },
    { key: 'markers', label: 'markers', swatch: 'dots' }
  ];
</script>

<div class="legend" role="group" aria-label="Curve layers">
  {#each LAYERS as layer}
    {@const active = toggles?.[layer.key] !== false}
    <button
      type="button"
      class="legend-item {active ? 'on' : 'off'}"
      aria-pressed={active}
      aria-label="Toggle {layer.label}"
      title="Toggle {layer.label}"
      onclick={() => onToggle?.(layer.key, !active)}
    >
      <span class="swatch {layer.swatch}" aria-hidden="true"></span>
      <span class="legend-label">{layer.label}</span>
    </button>
  {/each}
</div>

<style>
  .legend {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    font-size: 12px;
    color: #6e6b8a;
    margin: 8px 0 0;
  }
  .legend-item {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    padding: 8px;
    cursor: pointer;
    min-height: 44px;
    min-width: 44px;
    color: #6e6b8a;
  }
  .legend-item:focus-visible {
    outline: 2px solid #4f46e5;
    outline-offset: 2px;
  }
  .legend-item.off {
    color: #9c99b8;
  }
  .swatch {
    display: inline-block;
    width: 24px;
    height: 0;
  }
  .swatch.solid-indigo {
    border-top: 3px solid #4f46e5;
  }
  .swatch.dashed-green {
    border-top: 3px dashed #1fa36b;
  }
  .swatch.dashed-purple {
    border-top: 3px dashed #9333ea;
  }
  .swatch.dashed-amber {
    border-top: 3px dashed #e8930c;
  }
  .swatch.dots {
    width: 10px;
    height: 10px;
    border-radius: 9999px;
    background: #4f46e5;
    border: 2px solid #fff;
    box-shadow: 0 0 0 1px #4f46e5;
  }
  .legend-item.off .swatch {
    border-color: #9c99b8;
    background: #9c99b8;
  }
  @media (prefers-reduced-motion: reduce) {
    .legend-item {
      transition: none;
    }
  }
</style>
