<script>
  let { params, onParam } = $props();
</script>

<div class="sliders">
  {#each Object.entries(params) as [name, value]}
    <div class="slider-row">
      <label for="slider-{name}"><code>{name} = {value.toFixed(2)}</code></label>
      <input
        id="slider-{name}"
        type="range"
        min="-5"
        max="5"
        step="0.1"
        value={value}
        oninput={(e) => onParam(name, parseFloat(e.target.value))}
      />
      <input
        type="number"
        step="0.1"
        value={value}
        aria-label="{name} value"
        oninput={(e) => onParam(name, parseFloat(e.target.value))}
      />
    </div>
  {:else}
    <p><em>No parameters — use a name like <code>a</code> in the formula to get a slider.</em></p>
  {/each}
</div>
