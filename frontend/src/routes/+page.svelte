<script lang="ts">
  // Definiujemy kształt naszych danych z serwera
  type WynikObliczen = {
    wzor: string;
    pochodna: string;
    wlasnosci: string[];
  };

  // 1. Stan aplikacji
  let funkcja = $state("");
  let wynik = $state<WynikObliczen | null>(null);
  let ladowanie = $state(false);

  // 2. Uchwyt na płótno (canvas)
  let canvasWykresu = $state<HTMLCanvasElement | null>(null);

  // 3. Symulacja zapytania do Rusta (API)
  async function wyslijDoObliczenia() {
    if (funkcja.trim() === "") return;

    ladowanie = true;
    wynik = null;

    // Udajemy ładowanie z sieci
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Wypełniamy sztucznymi danymi
    wynik = {
      wzor: funkcja,
      pochodna: "2x + pochodna z " + funkcja,
      wlasnosci: ["dziedzina: R", "miejsca zerowe: brak"],
    };

    ladowanie = false;

    // TUTAJ W PRZYSZŁOŚCI WYWOŁAMY FUNKCJĘ WASM, NP:
    // jesli element canvas istnieje w HTML, przekaż go do Rusta
    // if (canvasWykresu) {
    //   wasz_modul_wasm.narysuj_wykres(canvasWykresu, wynik.wzor);
    // }
  }
</script>

<main>
  <h1>GeoGebra RS - Panel Obliczeniowy</h1>

  <div class="panel-wejsciowy">
    <input 
      type="text" 
      bind:value={funkcja} 
      placeholder="Wpisz wzór funkcji, np. x^2" 
    />
    <button onclick={wyslijDoObliczenia} disabled={ladowanie}>
      {ladowanie ? "Obliczanie..." : "Licz pochodną"}
    </button>
  </div>

  {#if wynik}
    <div class="wyniki">
      <h2>Rozwiązanie dla: {wynik.wzor}</h2>
      <p><strong>Pochodna:</strong> {wynik.pochodna}</p>
      
      <h3>Własności:</h3>
      <ul>
        {#each wynik.wlasnosci as cecha}
          <li>{cecha}</li>
        {/each}
      </ul>

      <h3>Wykres funkcji:</h3>
      <div class="pojemnik-na-wykres">
        <!-- bind:this przypisuje ten fizyczny element HTML do naszej zmiennej w Svelte -->
        <canvas bind:this={canvasWykresu}></canvas>
        <p class="placeholder-tekst">Miejsce na renderowanie wykresu z WASM</p>
      </div>
    </div>
  {/if}
</main>

<style>
  main {
    max-width: 800px;
    margin: 0 auto;
    padding: 20px;
    font-family: sans-serif;
  }
  .panel-wejsciowy {
    display: flex;
    gap: 10px;
    margin-bottom: 20px;
  }
  input {
    flex-grow: 1;
    padding: 10px;
    font-size: 16px;
  }
  button {
    padding: 10px 20px;
    font-size: 16px;
    cursor: pointer;
  }
  button:disabled {
    cursor: not-allowed;
    opacity: 0.7;
  }
  .wyniki {
    background: #f4f4f4;
    padding: 20px;
    border-radius: 8px;
  }
  /* Style dla naszego pustego pola na wykres */
  .pojemnik-na-wykres {
    width: 100%;
    height: 300px;
    background: #e0e0e0;
    border: 2px dashed #999;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    margin-top: 15px;
  }
  canvas {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
  }
  .placeholder-tekst {
    color: #666;
    font-weight: bold;
    z-index: 1;
  }
</style>