<script>
  let { value, error, onInput } = $props();

  let timer = null;
  function handle(e) {
    clearTimeout(timer);
    const v = e.target.value;
    timer = setTimeout(() => onInput(v), 120);
  }
</script>

<div class="bar">
  <label for="formula"><strong>f(x)&nbsp;=</strong></label>
  <input id="formula" type="text" value={value} oninput={handle} spellcheck="false" autocomplete="off" />
</div>
{#if error}
  <p class="error">
    {error.message}
    <span> (at characters {error.span[0]}–{error.span[1]})</span>
  </p>
{/if}
