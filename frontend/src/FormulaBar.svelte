<script>
  // R1 validation loop: typing revalidates synchronously (<50ms, no debounce)
  // via derive.js char-exact spans; Enter/Analyze fires WASM analysis only
  // when input is non-empty and error-free. Clear empties to the empty state.
  // Try chips fill-and-fire immediately.
  import { TRY_PRESETS } from './lib/derive.js';

  let {
    value = '',
    error = null,
    analyzing = false,
    onInput,
    onAnalyze,
    onClear,
    onTry
  } = $props();

  function handleInput(e) {
    onInput?.(e.target.value);
  }

  function handleKey(e) {
    if (e.key === 'Enter' && canAnalyze()) {
      e.preventDefault();
      onAnalyze?.();
    }
  }

  function canAnalyze() {
    return !analyzing && value.trim().length > 0 && !error;
  }

  function positionText() {
    if (!error || !error.span) return '';
    const [s, en] = error.span;
    if (s === en) return ` at position ${s}`;
    if (en === s + 1) return ` at position ${s}`;
    return ` at characters ${s}–${en}`;
  }

  function isActivePreset(p) {
    return value.trim() === p.formula;
  }
</script>

<div class="function-bar">
  <div class="bar-top">
    <span class="hint">Use x as your variable</span>
  </div>
  <div class="bar-row">
    <div class="input-wrap {error ? 'has-error' : ''}">
      <span class="prefix" aria-hidden="true">f(x) =</span>
      <input
        id="formula"
        type="text"
        value={value}
        oninput={handleInput}
        onkeydown={handleKey}
        spellcheck="false"
        autocomplete="off"
        autocapitalize="off"
        placeholder="e^(0.5x)"
        aria-label="Function of x"
        aria-invalid={error ? 'true' : 'false'}
        aria-describedby={error ? 'formula-error' : 'formula-examples'}
      />
      {#if value}
        <button
          class="clear"
          type="button"
          aria-label="Clear function input"
          title="Clear"
          onclick={() => onClear?.()}
        >×</button>
      {/if}
    </div>
    <button
      class="analyze"
      type="button"
      disabled={!canAnalyze()}
      onclick={() => onAnalyze?.()}
      aria-label="Analyze function"
    >
      {analyzing ? 'Analyzing…' : 'Analyze function ↗'}
    </button>
  </div>
  {#if error}
    <p class="error-text" id="formula-error" role="alert">
      {error.message}{positionText()}
    </p>
  {:else}
    <p class="examples" id="formula-examples">Examples: sin(x), x^2 e^x, log(x)</p>
  {/if}
  <div class="try-row" role="group" aria-label="Try an example">
    <span class="try-label">TRY ONE</span>
    {#each TRY_PRESETS as p}
      <button
        type="button"
        class="try-chip {isActivePreset(p) ? 'active' : ''}"
        onclick={() => onTry?.(p.formula)}
        aria-pressed={isActivePreset(p)}
      >{p.label}</button>
    {/each}
  </div>
</div>

<style>
  .function-bar {
    background: #ffffff;
    border: 1px solid #e6e5f1;
    border-radius: 14px;
    padding: 12px 16px;
  }
  .bar-top {
    display: flex;
    justify-content: flex-end;
    margin-bottom: 4px;
  }
  .hint {
    font-size: 12px;
    color: #6e6b8a;
  }
  .bar-row {
    display: flex;
    gap: 12px;
    align-items: stretch;
  }
  .input-wrap {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    border: 1px solid #e6e5f1;
    border-radius: 10px;
    padding: 0 8px;
    background: #fff;
    min-width: 0;
  }
  .input-wrap.has-error {
    border-color: #dc2626;
  }
  .prefix {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 15px;
    font-weight: 500;
    color: #1e1b33;
    white-space: nowrap;
  }
  .input-wrap input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 15px;
    font-weight: 500;
    padding: 10px 0;
    background: transparent;
    color: #1e1b33;
  }
  .clear {
    border: none;
    background: transparent;
    font-size: 18px;
    line-height: 1;
    color: #6e6b8a;
    cursor: pointer;
    padding: 8px;
    min-width: 44px;
    min-height: 44px;
  }
  .analyze {
    background: #4f46e5;
    color: #ffffff;
    border: none;
    border-radius: 10px;
    font-size: 14px;
    font-weight: 600;
    padding: 0 16px;
    cursor: pointer;
    box-shadow: 0 4px 14px rgba(79, 70, 229, 0.35);
    white-space: nowrap;
    min-height: 44px;
  }
  .analyze:hover:not(:disabled) {
    background: #4338ca;
  }
  .analyze:disabled {
    background: #9c99b8;
    box-shadow: none;
    cursor: default;
  }
  .error-text {
    color: #dc2626;
    font-size: 12px;
    margin: 8px 0 0;
  }
  .examples {
    color: #6e6b8a;
    font-size: 12px;
    margin: 8px 0 0;
  }
  .try-row {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-top: 12px;
    flex-wrap: wrap;
  }
  .try-label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: #6e6b8a;
  }
  .try-chip {
    background: #ffffff;
    color: #6e6b8a;
    border: 1px solid #e6e5f1;
    border-radius: 6px;
    font-size: 13px;
    padding: 8px 12px;
    cursor: pointer;
    min-height: 44px;
  }
  .try-chip:hover,
  .try-chip.active {
    background: #edecfd;
    color: #4f46e5;
  }
  @media (prefers-reduced-motion: reduce) {
    .analyze,
    .try-chip {
      transition: none;
    }
  }
</style>
