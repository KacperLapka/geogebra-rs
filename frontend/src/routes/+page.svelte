<script lang="ts">
  import { onMount } from 'svelte';

  // silnik graficzny
  import initPlot, { updateGraph, addOffset, zoom } from '#lib/pkg/wasm.js';
  import plotWasmUrl from '#lib/pkg/wasm_bg.wasm?url';

  // silnik matematyczny (nadal używany do wyświetlenia wzoru pochodnej w HTML)
  import initMath, { compute_derivative } from '#lib/pkg_math/math_engine.js';
  import mathWasmUrl from '#lib/pkg_math/math_engine_bg.wasm?url';

  type WynikObliczen = { wzor: string; pochodna: string; wlasnosci: string[] };

  let funkcja = $state("x^2");
  let wynik = $state<WynikObliczen | null>(null);
  let ladowanie = $state(false);
  let wasmGotowy = $state(false);

  // stan do obsługi przesuwania (pan) i zoomu na canvasie
  let canvasEl: HTMLCanvasElement;
  let isDragging = $state(false);
  let lastX = 0;
  let lastY = 0;

  onMount(() => {
    async function initWasm() {
      try {
        await initPlot(plotWasmUrl);
        await initMath(mathWasmUrl);
        wasmGotowy = true;
      } catch (e) {
        console.error("Błąd ładowania modułów WASM:", e);
      }
    }

    initWasm();

    // Nasłuch na wheel z { passive: false }, żeby e.preventDefault() blokowało przewijanie strony
    const handleWheel = (e: WheelEvent) => {
      e.preventDefault();
      if (!wasmGotowy) return;

      // kółko w górę (deltaY < 0) = przybliżenie, w dół = oddalenie
      const factor = e.deltaY < 0 ? 0.9 : 1.1;
      try {
        zoom(factor);
      } catch (err) {
        console.error("Błąd zoom:", err);
      }
    };

    canvasEl.addEventListener('wheel', handleWheel, { passive: false });
    return () => canvasEl.removeEventListener('wheel', handleWheel);
  });

  function handlePointerDown(e: PointerEvent) {
    if (!wasmGotowy) return;
    isDragging = true;
    lastX = e.clientX;
    lastY = e.clientY;
    canvasEl.setPointerCapture(e.pointerId);
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isDragging || !wasmGotowy) return;

    const rect = canvasEl.getBoundingClientRect();
    // Przeliczenie delty z pikseli na znormalizowany zakres płótna (-2..2)
    const dx = -((e.clientX - lastX) / rect.width) * 2.0;
    const dy = -((e.clientY - lastY) / rect.height) * 2.0;

    lastX = e.clientX;
    lastY = e.clientY;

    try {
      addOffset(dx, dy);
    } catch (err) {
      console.error("Błąd addOffset:", err);
    }
  }

  function handlePointerUp(e: PointerEvent) {
    if (!isDragging) return;
    isDragging = false;
    if (canvasEl.hasPointerCapture(e.pointerId)) {
      canvasEl.releasePointerCapture(e.pointerId);
    }
  }

  async function wyslijDoObliczenia() {
    if (funkcja.trim() === "") return;
    ladowanie = true;
    wynik = null;

    if (!wasmGotowy) {
      alert("Silniki jeszcze się ładują, poczekaj chwilę!");
      ladowanie = false;
      return;
    }

    try {
      const wyliczonaPochodna = compute_derivative(funkcja);

      wynik = {
        wzor: funkcja,
        pochodna: wyliczonaPochodna,
        wlasnosci: ["Obliczenia wykonane natywnie w Rust"],
      };

      // updateGraph przyjmuje teraz bezpośrednio string ze wzorem
      // (jeśli chcesz rysować wykres pochodnej zamiast samej funkcji, przekaż `wyliczonaPochodna`)
      updateGraph(funkcja);
    } catch (error) {
      alert("Błąd silnika matematycznego Rusta: Nieznany format wzoru.");
      console.error(error);
    }

    ladowanie = false;
  }
</script>

<main>
  <h1>GeoGebra RS - Panel Obliczeniowy</h1>

  <div class="panel-wejsciowy">
    <input type="text" bind:value={funkcja} placeholder="Wpisz wzór funkcji, np. x^2" />
    <button onclick={wyslijDoObliczenia} disabled={ladowanie}>
      {ladowanie ? "Obliczanie..." : "Licz pochodną"}
    </button>
  </div>

  <h3>Wykres pochodnej funkcji:</h3>
  <div class="pojemnik-na-wykres">
    <canvas
      id="canvas"
      width="400"
      height="400"
      bind:this={canvasEl}
      class:dragging={isDragging}
      onpointerdown={handlePointerDown}
      onpointermove={handlePointerMove}
      onpointerup={handlePointerUp}
      onpointercancel={handlePointerUp}
    ></canvas>
  </div>

  {#if wynik}
    <div class="wyniki">
      <h2>Rozwiązanie dla: {wynik.wzor}</h2>
      <p><strong>Pochodna:</strong> {wynik.pochodna}</p>
    </div>
  {/if}
</main>

<style>
  main { max-width: 800px; margin: 0 auto; padding: 20px; font-family: sans-serif; }
  .panel-wejsciowy { display: flex; gap: 10px; margin-bottom: 20px; }
  input { flex-grow: 1; padding: 10px; font-size: 16px; }
  button { padding: 10px 20px; font-size: 16px; cursor: pointer; }
  .wyniki { background: #f4f4f4; padding: 20px; border-radius: 8px; }
  .pojemnik-na-wykres { width: 100%; display: flex; justify-content: center; margin-top: 15px; }
  canvas {
    max-width: 600px !important;
    max-height: 400px !important;
    width: 100% !important;
    background: white;
    border: 1px solid #ccc;
    border-radius: 4px;
    margin: 0 auto;
    display: block;
    cursor: grab;
    touch-action: none;
  }
  canvas.dragging {
    cursor: grabbing;
  }
</style>