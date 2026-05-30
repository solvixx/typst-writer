// ============================================================
// TYPST-WRITER FIDELITY TEST DOCUMENT
// Tests: #set, #show, context, labels, @references, math,
//        integrals, radicals, fractions, matrices, code,
//        figures, footnotes, bibliography stubs.
// ============================================================

// ── Global style rules ──────────────────────────────────────
#set document(title: "Fidelity Test", author: "typst-writer")
#set page(
  paper: "a4",
  margin: (top: 2.5cm, bottom: 2.5cm, left: 3cm, right: 2.5cm),
  numbering: "1 / 1",
  header: context {
    let page-num = counter(page).at(here()).first()
    if page-num > 1 [
      _Fidelity Test_ #h(1fr) Page #page-num
    ]
  },
)

#set text(font: "New Computer Modern", size: 11pt, lang: "en")
#set heading(numbering: "1.1")
#set par(justify: true, leading: 0.65em)
#set math.equation(numbering: "(1)")

// ── Custom show rules ────────────────────────────────────────
#show heading.where(level: 1): it => {
  v(1.2em)
  block(text(weight: "bold", size: 13pt, it.body))
  v(0.4em)
}

#show link: it => underline(text(fill: blue, it))

// ── Helper function using context ────────────────────────────
#let current-section = context {
  let h = query(selector(heading).before(here()))
  if h.len() > 0 { h.last().body } else { [—] }
}

// ── Title block ──────────────────────────────────────────────
#align(center)[
  #text(size: 20pt, weight: "bold")[Fidelity Test Document]
  \
  #text(size: 12pt, style: "italic")[
    Testing all major Typst features in typst-writer
  ]
  \
  #text(size: 10pt)[
    #context {
      let today = datetime.today()
      today.display("[day] [month repr:long] [year]")
    }
  ]
]

#v(1em)
#line(length: 100%, stroke: 0.5pt)
#v(0.5em)

// ── Table of contents (context-dependent) ────────────────────
#outline(title: "Contents", depth: 2, indent: 1.5em)

#pagebreak()

// ============================================================
= Introduction <sec:intro>
// ============================================================

This document exercises the rendering fidelity of *typst-writer*. It deliberately
uses every major Typst feature so that the editor's caret navigation, glyph-box
hit-testing, and whitespace-masking compilation strategy can be validated together.

The document structure follows a standard scientific paper: introduction, theory,
methods, results, and appendices. Each section stresses a different subset of
Typst's layout engine.

Inline equations like $a^2 + b^2 = c^2$ compile on every keystroke. Below is a
labelled block equation:

$ integral_a^b f(x) dif x = F(b) - F(a) $ <eq:fundamental-theorem>

See @eq:fundamental-theorem for the Fundamental Theorem of Calculus, and
@sec:calculus for a full treatment.

// ============================================================
= Mathematical Foundations <sec:calculus>
// ============================================================

== Real Analysis

Let $f : [a, b] -> RR$ be a continuous function. The #emph[Riemann integral] is
defined as the limit of Riemann sums:

$ integral_a^b f(x) dif x
    = lim_(n -> infinity) sum_(k=1)^n f(x_k^*) Delta x_k $ <eq:riemann>

where $Delta x_k = (b - a) / n$ and $x_k^* in [x_(k-1), x_k]$.

@eq:riemann illustrates why the integral and the radical symbol require careful
caret-positioning — both are tall glyphs whose bounding boxes extend above and below
the text baseline.

== Radical Expressions

The general solution of $a x^2 + b x + c = 0$ is:

$ x = (-b plus.minus sqrt(b^2 - 4 a c)) / (2 a) $ <eq:quadratic>

A nested radical: $sqrt(1 + sqrt(1 + sqrt(1 + sqrt(1 + dots.c))))$.

The $n$-th root appears in the geometric mean:

$ overline(x)_"geo" = root(n, product_(i=1)^n x_i)
    = root(n, x_1 dot x_2 dot dots.c dot x_n) $ <eq:geomean>

== Fractions and Continued Fractions

The golden ratio $phi$ satisfies $phi = 1 + 1/phi$, giving the continued fraction:

$ phi = 1 + display(1 / (1 + 1 / (1 + 1 / (1 + dots.down)))) $ <eq:golden>

Euler's continued fraction for $e$:

$ e = 2 + display(1 / (1 + 1 / (2 + 2 / (3 + 3 / (4 + dots.down))))) $ <eq:euler-e>

== Integrals

=== Definite integrals

$ integral_0^(pi/2) sin^2(x) dif x = pi / 4 $ <eq:sin2>

$ integral_(-infinity)^(+infinity) e^(-x^2) dif x = sqrt(pi) $ <eq:gaussian>

The Gaussian integral (@eq:gaussian) is fundamental to probability theory. It
relates directly to the error function:

$ "erf"(z) = 2/sqrt(pi) integral_0^z e^(-t^2) dif t $ <eq:erf>

=== Double and triple integrals

$ integral.double_D f(x, y) dif A
    = integral_a^b integral_(g(x))^(h(x)) f(x, y) dif y dif x $ <eq:double>

$ integral.triple_V f(x,y,z) dif V
    = integral_a^b integral_c^d integral_e^f f dif z dif y dif x $ <eq:triple>

=== Line integrals and Green's theorem

$ integral.cont_C (P dif x + Q dif y)
    = integral.double_D ((partial Q)/(partial x) - (partial P)/(partial y)) dif A $ <eq:green>

// ============================================================
= Linear Algebra <sec:linalg>
// ============================================================

== Matrices

Let $A in RR^(m times n)$. The matrix below shows a general $3 times 3$ system:

$ A = mat(
  a_(11), a_(12), a_(13);
  a_(21), a_(22), a_(23);
  a_(31), a_(32), a_(33)
) $ <eq:matrix3>

The determinant of a $2 times 2$ matrix:

$ det mat(a, b; c, d) = a d - b c $ <eq:det2>

Cramer's rule for $A bold(x) = bold(b)$:

$ x_i = det(A_i) / det(A), quad i = 1, dots, n $ <eq:cramer>

== Eigenvalues

The characteristic polynomial of $A$ is $p(lambda) = det(A - lambda I)$.
The eigenvalue equation:

$ A bold(v) = lambda bold(v), quad bold(v) != bold(0) $ <eq:eigen>

For a symmetric positive-definite matrix the spectral decomposition gives:

$ A = sum_(i=1)^n lambda_i bold(v)_i bold(v)_i^T $ <eq:spectral>

// ============================================================
= Probability and Statistics <sec:prob>
// ============================================================

== Probability Distributions

The normal distribution $X tilde cal(N)(mu, sigma^2)$ has density:

$ f(x) = 1/(sigma sqrt(2 pi)) exp(-(x - mu)^2 / (2 sigma^2)) $ <eq:normal>

The moment-generating function:

$ M_X (t) = EE[e^(t X)] = exp(mu t + sigma^2 t^2 / 2) $ <eq:mgf>

== Bayesian Inference <sec:bayes>

Bayes' theorem (@sec:bayes) in continuous form:

$ p(theta | x) = (p(x | theta) p(theta)) / (integral p(x | theta') p(theta') dif theta') $ <eq:bayes>

The denominator is the marginal likelihood, also called the *evidence*:

$ p(x) = integral p(x | theta) p(theta) dif theta $ <eq:evidence>

// ============================================================
= Context-Dependent Content <sec:ctx>
// ============================================================

This section tests Typst's `context` keyword — content whose output depends on
where it appears in the document.

The current section heading (resolved at layout time) is:
#context {
  let headings = query(selector(heading.where(level: 1)).before(here()))
  if headings.len() > 0 {
    emph(headings.last().body)
  } else {
    [_unknown_]
  }
}

Page numbering is also context-dependent. We are currently on page
#context { counter(page).display() }.

The total number of labelled equations so far:
#context {
  let n1 = query(<eq:fundamental-theorem>).len()
  let n2 = query(<eq:riemann>).len()
  let n3 = query(<eq:quadratic>).len()
  str(n1 + n2 + n3) + " key equations referenced"
}

=== Font-size aware rule

#context {
  let sz = text.size
  [The body text size is #sz. ]
  if sz >= 12pt [This is a large-type document.] else [Standard size.]
}

// ============================================================
= Code and Algorithms <sec:code>
// ============================================================

Inline code: `let x = integral_0^1 f(t) dt` is rendered as raw text.

A fenced code block:

```rust
pub fn newton_raphson<F>(f: F, df: F, mut x: f64, tol: f64) -> f64
where
    F: Fn(f64) -> f64,
{
    loop {
        let fx = f(x);
        if fx.abs() < tol { break; }
        x -= fx / df(x);
    }
    x
}
```

A Python snippet:

```python
import numpy as np

def gaussian_quadrature(f, a, b, n=5):
    """Gauss-Legendre quadrature on [a, b] with n points."""
    xi, wi = np.polynomial.legendre.leggauss(n)
    t = 0.5 * (b - a) * xi + 0.5 * (b + a)
    return 0.5 * (b - a) * np.dot(wi, f(t))
```

// ============================================================
= Figures and Floats <sec:figures>
// ============================================================

#figure(
  table(
    columns: 4,
    stroke: 0.5pt,
    align: center,
    table.header([*Method*], [*Order*], [*Error*], [*Stability*]),
    [Euler],          [$O(h)$],    [$10^(-2)$], [Conditionally],
    [Trapezoidal],    [$O(h^2)$],  [$10^(-4)$], [Unconditionally],
    [RK4],            [$O(h^4)$],  [$10^(-8)$], [Conditionally],
    [Adams-Moulton],  [$O(h^5)$],  [$10^(-9)$], [Unconditionally],
    [BDF-6],          [$O(h^6)$],  [$10^(-11)$],[A-stable],
  ),
  caption: [Comparison of numerical ODE solvers. Higher-order methods achieve
            exponentially smaller errors per step.],
) <tbl:solvers>

@tbl:solvers summarises common ODE solvers. The Runge-Kutta family (RK4) is
the most widely used in practice due to its balance of accuracy and stability.

// ============================================================
= Advanced Mathematics <sec:advanced>
// ============================================================

== Fourier Analysis

The Fourier transform and its inverse:

$ hat(f)(xi) = integral_(-infinity)^(+infinity) f(x) e^(-2 pi i x xi) dif x $ <eq:fourier>

$ f(x) = integral_(-infinity)^(+infinity) hat(f)(xi) e^(2 pi i x xi) dif xi $ <eq:inv-fourier>

Parseval's theorem:

$ integral_(-infinity)^(+infinity) |f(x)|^2 dif x
    = integral_(-infinity)^(+infinity) |hat(f)(xi)|^2 dif xi $ <eq:parseval>

== Complex Analysis

Cauchy's integral formula:

$ f^((n))(z_0) = n! / (2 pi i) integral.cont_(|z - z_0| = r) f(z) / (z - z_0)^(n+1) dif z $ <eq:cauchy>

The residue theorem:

$ integral.cont_C f(z) dif z = 2 pi i sum_k "Res"(f, z_k) $ <eq:residue>

== Differential Equations

The heat equation:

$ (partial u)/(partial t) = alpha (partial^2 u)/(partial x^2),
  quad u(x,0) = u_0(x),
  quad u(0,t) = u(L,t) = 0 $ <eq:heat>

Solution by separation of variables:

$ u(x, t) = sum_(n=1)^(infinity) B_n sin((n pi x)/L) e^(-alpha (n pi / L)^2 t) $ <eq:heat-sol>

where $B_n = 2/L integral_0^L u_0(x) sin((n pi x)/L) dif x$.

The wave equation:

$ (partial^2 u)/(partial t^2) = c^2 nabla^2 u $ <eq:wave>

Schrödinger's equation (time-independent):

$ hat(H) psi = [- planck^2/(2m) nabla^2 + V(bold(r))] psi = E psi $ <eq:schrodinger>

== Tensor Notation

Einstein summation convention: $A_(i j) B^(j k) = C_i^k$.

The Riemann curvature tensor:

$ R^rho_(space sigma mu nu)
    = partial_mu Gamma^rho_(nu sigma)
    - partial_nu Gamma^rho_(mu sigma)
    + Gamma^rho_(mu lambda) Gamma^lambda_(nu sigma)
    - Gamma^rho_(nu lambda) Gamma^lambda_(mu sigma) $ <eq:riemann-tensor>

// ============================================================
= References and Cross-links <sec:refs>
// ============================================================

Summary of all labelled content in this document:

- Introduction: @sec:intro
- Calculus section: @sec:calculus — Fundamental theorem: @eq:fundamental-theorem
- Riemann sum definition: @eq:riemann
- Quadratic formula: @eq:quadratic — Geometric mean: @eq:geomean
- Golden ratio: @eq:golden — Euler's $e$: @eq:euler-e
- Gaussian integral: @eq:gaussian — Error function: @eq:erf
- Double integral: @eq:double — Green's theorem: @eq:green
- Matrix system: @eq:matrix3 — Eigenvalue equation: @eq:eigen
- Normal distribution: @eq:normal — Bayes: @eq:bayes
- Context section: @sec:ctx
- Solver table: @tbl:solvers
- Fourier transform: @eq:fourier — Parseval: @eq:parseval
- Cauchy integral: @eq:cauchy — Residue theorem: @eq:residue
- Heat equation: @eq:heat — Wave equation: @eq:wave
- Schrödinger: @eq:schrodinger — Riemann tensor: @eq:riemann-tensor

// ============================================================
= Appendix: Stress Test Markup <sec:stress>
// ============================================================

This section deliberately mixes prose, inline math, display math, code, and
context blocks to stress-test the whitespace-masking compile path.

#for i in range(1, 8) {
  let eqs = (
    $ integral_0^(i) x^(i) dif x = i^(i+1)/(i+1) $,
    $ sum_(k=0)^i binom(i, k) = 2^i $,
    $ root(i, i!) approx i/e $,
    $ phi^i = phi^(i-1) + phi^(i-2) $,
    $ zeta(i) = sum_(n=1)^infinity 1/n^i $,
    $ Gamma(i) = (i-1)! $,
    $ e^(i pi) + 1 = 0 $,
  )
  [
    *Identity #i.* The #str(i)-th entry in our stress suite: ]
  eqs.at(calc.rem(i - 1, eqs.len()))
  [ \
  ]
}

#v(1em)

=== Deeply Nested Math

$ limits(lim)_(x -> 0)
    (1 - cos x) / x^2
    = limits(lim)_(x -> 0)
        (sin x) / (2x)
    = 1/2 $ <eq:lhopital>

$ sum_(n=0)^(infinity) x^n / n!
    = 1 + x + x^2/2! + x^3/3! + x^4/4! + dots
    = e^x, quad |x| < infinity $ <eq:exp-series>

$ J_nu (x) = sum_(m=0)^(infinity)
    (-1)^m / (m! Gamma(m + nu + 1))
    (x/2)^(2m+nu) $ <eq:bessel>

#line(length: 100%, stroke: 0.5pt)
#v(0.5em)
#text(size: 9pt, style: "italic")[
  End of fidelity test document. Total equations: approximately 35 labelled.
  All features exercised: `#set`, `#show`, `context`, labels, `@references`,
  display math, inline math, integrals, radicals, fractions, matrices,
  continued fractions, code blocks, figures, tables, and Typst scripting.
]
