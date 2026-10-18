# Glossary: GeoGebra RS

* **AST (Abstract Syntax Tree):** Hierarchical in-memory tree representation of mathematical expressions produced by the parsing engine from text formulas.
* **Symbolic Derivative:** Analytical calculation of derivative functions (e.g., $\frac{d}{dx}(\sin(x)) = \cos(x)$) via algebraic calculus transformation rules rather than finite-difference approximations.
* **Numerical Fallback:** Discretized numerical algorithms (e.g., Brent's root-finding method, central finite differences) invoked when analytical closed forms are transcendental or undefined.
* **wgpu:** Safe, portable Rust graphics library targeting WebGPU natively and in web browsers via WebAssembly, featuring automated WebGL2 fallback.
* **Viewport:** The active bounding rectangular coordinate window $[x_{\min}, x_{\max}] \times [y_{\min}, y_{\max}]$ mapped to the screen canvas pixel buffer.
* **Critical Point:** Coordinates where $f'(x) = 0$ (stationary points) or where $f'(x)$ fails to exist.
* **Inflection Point:** Coordinates on the curve where concavity changes sign, occurring where $f''(x) = 0$ and $f''(x)$ alters sign across the point.
* **Asymptote:** A line that the graph of a curve approaches arbitrarily closely; includes vertical ($x = c$), horizontal ($y = L$), and slant/oblique ($y = mx + b$) asymptotes.
