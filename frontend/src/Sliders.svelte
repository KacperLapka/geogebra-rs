<script>
  // R2 param sliders: one row per detected parameter, eval-only
  // (`updateParam` + resample in the same frame, never `setFunction`).
  // Hidden when the expression has no parameters. Keyboard: native range
  // arrows move ±0.1 (step), PageUp/Down ±1, Home/End to edges.
  // Live updates flow via `onParam`; screen-reader commits flow via
  // `onCommit` (release only) so aria-live never announces every frame.
  let { params, onParam, onCommit } = $props();

  function forward(name, raw) {
    const value = typeof raw === 'number' ? raw : parseFloat(raw);
    if (!Number.isFinite(value)) return;
    onParam(name, value);
  }

  function commit(name, raw) {
    const value = typeof raw === 'number' ? raw : parseFloat(raw);
    if (!Number.isFinite(value)) return;
    onCommit?.(name, value);
  }

  const fmt = (v) => Number(v).toFixed(2);
  const names = () => Object.keys(params ?? {});
</script>

{#if names().length > 0}
  <div class="param-block" aria-label="Parameters">
    {#each Object.entries(params) as [name, value]}
      <div class="slider-row">
        <label for="slider-{name}"><code>{name} = {fmt(value)}</code></label>
        <input
          id="slider-{name}"
          type="range"
          min="-5"
          max="5"
          step="0.1"
          value={value}
          aria-label="{name} slider"
          title="{name} = {value}"
          oninput={(e) => forward(name, e.target.value)}
          onchange={(e) => commit(name, e.target.value)}
        />
        <input
          type="number"
          step="0.1"
          value={value}
          aria-label="{name} value"
          oninput={(e) => forward(name, e.target.value)}
          onchange={(e) => commit(name, e.target.value)}
        />
      </div>
    {/each}
  </div>
{/if}

<style>
  .param-block {
    margin-top: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .slider-row {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .slider-row input[type='range'] {
    flex: 1;
    accent-color: #4f46e5;
  }
  .slider-row code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 13px;
    font-weight: 600;
  }
</style>
