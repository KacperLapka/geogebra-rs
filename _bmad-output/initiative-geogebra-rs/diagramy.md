# Diagramy architektury GeoGebra

---

## 1. Widok deweloperski — UML Component Diagram

Wybrano wariant deweloperski (zamiast logicznego Class), bo istotą ADR jest podział na crate'y / moduły WASM.

```mermaid
flowchart TB
    subgraph FE["Frontend SvelteKit (+page.svelte)"]
        UI["+page.svelte\nfunkcja, wynik, wasmGotowy\nwyslijDoObliczenia()"]
    end

    subgraph MW["math-engine.wasm\n(cdylib)"]
        MR["lib.rs\nPointResult{x,y}\ncompute_given_function()\nevaluate_function()\ncompute_derivative()"]
        F["formulac::Builder"]
        S["symb_anafis::diff"]
        MR --> F
        MR --> S
    end

    subgraph PW["plot-engine.wasm\n(cdylib)"]
        WI["wasm_interface.rs\nGraphState{changed,path}\nupdateGraph(f32[1000])"]
        GR["graph.rs\npoints_raw_to_path()\npath_to_vertex()\nMyVertex"]
        LY["lyon_tessellation\nPath / StrokeTessellator\nStrokeOptions(0.008)"]
        ST["lib.rs State/App\nwgpu + winit\nGRAPH_STATE"]
        WI --> GR
        GR --> LY
        ST --> GR
    end

    subgraph API["api (bin, Axum)"]
        RT["main.rs Router\nPOST /api/workspaces\nGET /api/workspaces/{slug}"]
        SR["store.rs Store{PgPool}\nput() / get() / mint_slug()"]
        RT --> SR
    end

    DB[("Postgres\nworkspaces(slug PK, blob TEXT)")]

    UI -- "evaluate_function(expr,-10,10): Array<PointResult>\ncompute_derivative(expr): String" --> MR
    UI -- "updateGraph(Float32Array[1000] y)" --> WI
    WI -- "Path → VertexBuffers → VBO/IBO" --> ST
    UI -- "POST/GET JSON {slug}" --> RT
    SR -- "INSERT / SELECT blob" --> DB
```

---

## 2. Widok procesu — UML Communication Diagram (współpraca obiektów, ten sam przepływ)

```mermaid
flowchart LR
    U(["Użytkownik"]) -- "1: wzór, klik" --> P["P:+page"]
    P -- "2: compute_derivative(f)" --> M["M:math-engine"]
    M -- "3: String" --> P
    P -- "4: evaluate_function(f',-10,10)" --> M
    M -- "5: Array<PointResult> [x,y]" --> P
    P -- "6: Float32Array[1000] y" --> PL["PL:plot-engine\nupdateGraph"]
    PL -- "7: Path" --> GS["GS:GRAPH_STATE"]
    GS -- "8: VertexBuffers (lyon 0.008)" --> ST["ST:State"]
    ST -- "9: draw()" --> GPU(["GPU/wgpu"])
    GPU -- "10: klatka" --> U
```


---

## 3a. Diagram danych — ERD

```mermaid
erDiagram
    workspaces {
        TEXT slug PK
        TEXT blob
    }
```

`migrations/20261027000000_create_workspaces.sql:1-4`. Jedna tabela, bez FK. `slug` — 8 znaków `[A-Za-z0-9-_]` (`store.rs:11-15`, `routes.rs:11`), `blob` — `serde_json::Value` serializowany do TEXT (`store.rs:72,93`). Brak `created_at/expires_at`, brak `JSONB`, brak wersji schematu — stąd otwarta retencja z PRD.

---

## 3b. Diagram danych — DFD poziom 0/1

```mermaid
flowchart LR
    U((Użytkownik)) -- "wzór f(x)" --> P1["1. Wprowadzanie\n+page.svelte"]
    P1 -- "expr_str" --> P2["2. Obliczenia\nmath-engine"]
    P2 -- "pochodna String" --> P1
    P2 -- "Array<PointResult>" --> P1
    P1 -- "Float32Array[1000] y" --> P3["3. Teselacja\nlyon Path→trójkąty"]
    P3 -- "VertexBuffers" --> P4["4. Render\nwgpu/winit"]
    P4 -- "klatka" --> U
    P1 -- "JSON workspace" --> P5["5. Persystencja\nAxum Store"]
    P5 -- "INSERT/SELECT blob" --> D1[("D1 workspaces")]
    D1 --> P5
    P5 -- "{slug} / JSON" --> P1
```
