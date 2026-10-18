<script>
  // R2 reactive slider binding: every `oninput` forwards to
  // `update_param(name, value)` and resamples within the same frame.
  // No debounce, no `onchange` lag — the curve follows the drag.
  // Non-finite parses are dropped here; App.handleParam guards again.
  let { params, onParam } = $props();

  function forward(name, raw) {
    const value = typeof raw === 'number' ? raw : parseFloat(raw);
    if (!Number.isFinite(value)) return;
    onParam(name, value);
  }

  const fmt = (v) => Number(v).toFixed(2);
</script>

<div class="sliders">
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
      />
      <input
        type="number"
        step="0.1"
        value={value}
        aria-label="{name} value"
        oninput={(e) => forward(name, e.target.value)}
      />
    </div>
  {:else}
    <p><em>No parameters — use a name like <code>a</code> in the formula to get a slider.</em></p>
  {/each}
</div>
