#set page(paper: "a4", margin: 2cm, numbering: "1 / 1")
#set text(font: "Linux Libertine", size: 10pt)
#set heading(numbering: "1.1.")
#set math.equation(numbering: "(1)")

// Define a custom counter for exercises
#let exercise-counter = counter("exercise")

#outline(indent: auto)

= Introduction <intro>
This document is a massive stress-test for the Typst Writer application, consisting of 500 pages.
It validates:
- **Performance:** 60fps scrolling through 500 pages.
- **Fidelity:** Correct rendering of complex contexts and set rules.
- **Interactivity:** Accurate caret placement and selection in large documents.
- **Cross-References:** Resolving labels across hundreds of pages.

Refer to @final-chapter for the conclusion, or see the first equation @eq-1.

#for i in range(1, 501) [
  == Chapter #i #label("chap-" + str(i))
  #exercise-counter.step()
  
  *Exercise #context exercise-counter.display():* In this section, we analyze the properties of chapter #i. 
  
  #lorem(120)
  
  The fundamental identity for this page is:
  $ Phi_#i (x) = integral_0^#i e^(-t^2) d t + sum_(n=1)^#i 1/n^2 $ #label("eq-" + str(i))
  
  #context [
    #let current-page = here().page()
    #let total-pages = counter(page).final().at(0)
    *Telemetry:* Processing page #current-page of #total-pages. 
    #if i > 10 [
      Compare this result with @{"eq-" + str(i - 5)}.
    ]
  ]
  
  #if calc.rem(i, 3) == 0 [
    #grid(
      columns: (1fr, 1fr),
      gutter: 10pt,
      rect(width: 100%, height: 3cm, fill: blue.lighten(95%), stroke: 0.5pt + blue)[
        #set align(center + horizon)
        Grid Item A (#i)
      ],
      circle(radius: 1.5cm, fill: red.lighten(95%), stroke: 0.5pt + red)[
        #set align(center + horizon)
        #i
      ]
    )
  ] else if calc.rem(i, 3) == 1 [
    #block(
      width: 100%,
      inset: 8pt,
      fill: gray.lighten(90%),
      stroke: (left: 4pt + orange),
    )[
      #lorem(30)
    ]
  ]
  
  #pagebreak(weak: true)
]

= Final Conclusion <final-chapter>
We have successfully reached the end of the 500-page test.

*Global Stats:*
- Total Chapters: #context counter(heading).final().at(-1, default: 0)
- Total Exercises: #context exercise-counter.final().at(0, default: 0)
- Total Pages: #context counter(page).final().at(0, default: 0)

Back to @intro.
