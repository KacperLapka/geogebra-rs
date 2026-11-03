<script lang="ts">
  import { onMount } from 'svelte';
  
  // silnik graficzny
  import initPlot, { updateGraph } from '#lib/pkg/wasm.js';
  import plotWasmUrl from '#lib/pkg/wasm_bg.wasm?url';

  // silnik matematyczny
  import initMath, { compute_derivative, evaluate_function } from '#lib/pkg_math/math_engine.js';
  import mathWasmUrl from '#lib/pkg_math/math_engine_bg.wasm?url';

  type WynikObliczen = { wzor: string; pochodna: string; wlasnosci: string[] };

  let funkcja = $state("x^2");
  let wynik = $state<WynikObliczen | null>(null);
  let ladowanie = $state(false);
  
  // zmienna informująca, czy oba moduły WASM zostały już załadowane
  let wasmGotowy = $state(false);

  onMount(async () => {
    try {
      // inicjalizacja obu modułów WASM jeden po drugim
      await initPlot(plotWasmUrl);
      await initMath(mathWasmUrl);
      
      wasmGotowy = true;
    } catch (e) {
      console.error("Błąd ładowania modułów WASM:", e);
    }
  });

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
      // liczymu pochodną funkcji w Rust
      const wyliczonaPochodna = compute_derivative(funkcja);
      
      wynik = {
        wzor: funkcja,
        pochodna: wyliczonaPochodna,
        wlasnosci: ["Obliczenia wykonane natywnie w Rust"],
      };

      // pobieramy punkty do wykresu z Rusta, w zakresie od -10 do 10
      const punktyZRust = evaluate_function(wyliczonaPochodna, -10.0, 10.0);

      // tworzymy Float32Array, bo WebGPU wymaga takiej tablicy do rysowania
      const punktyDoWykresu = new Float32Array(1000);
      for (let i = 0; i < 1000; i++) {
        // punktyZRust[i] to obiekt PointResult {x, y}, bierzemy z niego tylko y
        punktyDoWykresu[i] = punktyZRust[i].y; 
      }

      // wysyłamy gotową tablicę do wyrysowania w WebGPU
      updateGraph(punktyDoWykresu);

    } catch (error) {
      // w ramach błędnego inputy, wyświetlamy alert i logujemy błąd w konsoli
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

  <!-- canvas został przeniesiony poza blok {#if}, żeby Rust znalazł go od razu przy starcie strony -->
  <h3>Wykres funkcji:</h3>
  <div class="pojemnik-na-wykres">
    <canvas id="canvas" width="400" height="400"></canvas>
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
    /* Wymuszamy maksymalne wymiary */
    max-width: 600px !important;
    max-height: 400px !important;
    width: 100% !important;
    background: white; 
    border: 1px solid #ccc; 
    border-radius: 4px; 
    margin: 0 auto;
    display: block;
  }
</style>