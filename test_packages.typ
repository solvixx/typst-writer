// ============================================================
// TYPST-WRITER PACKAGE FIDELITY TEST
// Tests: @local package, @preview packages (cetz, equate, codly)
// ============================================================

// ── Local package ────────────────────────────────────────────
#import "@local/tw-utils:0.1.0": theorem, proof, callout, vec, abs, sci

// ── Online/cached preview packages ───────────────────────────
#import "@preview/cetz:0.3.4"          // Drawing / diagrams
#import "@preview/equate:0.3.2": equate // Better equation numbering
#import "@preview/codly:1.3.0": *       // Syntax-highlighted code blocks

// ── Page setup ───────────────────────────────────────────────
#set page(paper: "a4", margin: 2cm, numbering: "1")
#set text(font: "New Computer Modern", size: 11pt)
#set heading(numbering: "1.")
#set math.equation(numbering: "(1)")

// Activate equate for per-line sub-equation numbering
#show: equate.with(breakable: true, sub-numbering: true)

// Activate codly for all raw blocks
#show: codly-init.with()
#codly(languages: (
  rust: (name: "Rust",   icon: none, color: rgb("#CE422B")),
  py:   (name: "Python", icon: none, color: rgb("#3572A5")),
))

// ── Title ─────────────────────────────────────────────────────
#align(center)[
  #text(18pt, weight: "bold")[Package Fidelity Test]
  \
  #text(11pt, style: "italic")[
    Testing `@local/tw-utils:0.1.0`, `@preview/cetz`, `@preview/equate`, `@preview/codly`
  ]
]
#v(0.8em)
#line(length: 100%, stroke: 0.5pt)
#v(0.4em)

// ============================================================
= Local Package: `tw-utils` <sec:local>
// ============================================================

The `@local/tw-utils` package provides theorem/proof blocks, coloured callouts,
and math shorthands. All are resolved from the local filesystem — no network access.

// ── Theorem / proof ──────────────────────────────────────────
#theorem("Pythagorean")[
  For a right triangle with legs $a, b$ and hypotenuse $c$:
  $ a^2 + b^2 = c^2 $
]

#proof[
  Drop a perpendicular from the right-angle vertex to the hypotenuse
  and apply similar triangles. $square$
]

#theorem("Cauchy-Schwarz")[
  For vectors $bold(u), bold(v) in RR^n$:
  $ abs(bold(u) dot bold(v)) <= abs(bold(u)) abs(bold(v)) $
]

// ── Callout boxes ─────────────────────────────────────────────
#callout(color: blue, title: "Info")[
  This callout uses `tw-utils.callout`. Background fill and border colour
  are derived automatically from the supplied `color` argument.
]

#callout(color: red, title: "Warning")[
  Always verify that `@local` package paths are accessible to the running
  `SimpleWorld` instance before compiling large documents.
]

// ── Math shorthands ───────────────────────────────────────────
Scientific notation via `sci()`: the speed of light is $c = sci(3, 8)$ m/s.

Vector shorthand: Newton's second law $vec(F) = m vec(a)$.

// ============================================================
= Online Package: `cetz` — Diagrams <sec:cetz>
// ============================================================

`@preview/cetz:0.3.4` is resolved from the local cache at
`~/.cache/typst/packages/preview/cetz/0.3.4/`. No download occurs at runtime.

#figure(
  cetz.canvas({
    import cetz.draw: *

    // Coordinate axes
    set-style(stroke: (thickness: 0.8pt))
    line((-0.3, 0), (4.5, 0), mark: (end: ">"))
    line((0, -0.3), (0, 3.2), mark: (end: ">"))
    content((4.6, 0), $x$)
    content((0, 3.3), $y$)

    // Parabola y = x²/4 sampled at integer x
    let pts = range(0, 5).map(x => (x, x * x / 4))
    hobby(..pts, stroke: blue + 1.2pt)

    // Labels
    for (x, y) in pts {
      circle((x, y), radius: 0.06, fill: blue)
    }
    content((2.2, 1.5), text(fill: blue)[$y = x^2/4$])

    // Right-angle marker at origin
    rect((0, 0), (0.2, 0.2), stroke: 0.5pt)
  }),
  caption: [Parabola $y = x^2/4$ plotted with `@preview/cetz`.],
) <fig:parabola>

// ============================================================
= Online Package: `equate` — Aligned Equations <sec:equate>
// ============================================================

`@preview/equate` adds sub-numbering (e.g. (2a), (2b)) to multi-line equations.

$ x + y &= 10 \
  x - y &= 4  $ <eq:system>

From @eq:system: $x = 7, y = 3$.

A system with three unknowns:

$ a + b + c &= 6  \
  a + 2b    &= 7  \
  2a - b    &= 1  $ <eq:sys3>

Solving @eq:sys3 by Gaussian elimination gives $a = 1, b = 3, c = 2$.

// ============================================================
= Online Package: `codly` — Highlighted Code <sec:codly>
// ============================================================

`@preview/codly` wraps all raw code blocks with line numbers and language badges.

```rust
/// Integrate f over [a, b] using adaptive Simpson's rule.
pub fn adaptive_simpson<F: Fn(f64) -> f64>(
    f: &F, a: f64, b: f64, tol: f64,
) -> f64 {
    let m = (a + b) / 2.0;
    let fa = f(a); let fm = f(m); let fb = f(b);
    let s = (b - a) / 6.0 * (fa + 4.0 * fm + fb);
    let s2 = (b - a) / 12.0 * (fa + 4.0 * f((a + m) / 2.0) + 2.0 * fm
                                  + 4.0 * f((m + b) / 2.0) + fb);
    if (s2 - s).abs() < 15.0 * tol { s2 } else {
        adaptive_simpson(f, a, m, tol / 2.0)
        + adaptive_simpson(f, m, b, tol / 2.0)
    }
}
```

```py
import numpy as np
from scipy.integrate import quad

# Gaussian integral: ∫_{-∞}^{∞} exp(-x²) dx = √π
result, error = quad(lambda x: np.exp(-x**2), -np.inf, np.inf)
print(f"Result: {result:.10f}  (exact: {np.sqrt(np.pi):.10f})")
```

// ============================================================
= Cross-references <sec:refs>
// ============================================================

- @sec:local — local package theorem/proof/callout
- @sec:cetz — diagram with cetz (@fig:parabola)
- @sec:equate — aligned equations @eq:system and @eq:sys3
- @sec:codly — syntax-highlighted code blocks

#v(1em)
#line(length: 100%, stroke: 0.5pt)
#text(size: 9pt, style: "italic")[
  Package paths resolved by `SimpleWorld.resolve_package_file()`:
  - `@local` → `~/.local/share/typst/packages/local/`
  - `@preview` → `~/.cache/typst/packages/preview/`
  No network requests are made during compilation.
]
