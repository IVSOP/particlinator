#import "@preview/typslides:1.2.6": *

#show: typslides.with(
  ratio: "16-9",
  theme: "bluey",
)

#set text(lang: "pt", region: "pt", font: "IBM Plex Sans")
#set par(justify: true)

#set raw(lang: "rs")

#front-slide(
  title: [Visualização em Tempo Real],
  subtitle: [Apresentação Trabalho Prático],
  authors: [Ivan Ribeiro, Francisco Ferreira e Júlio Pinto],
)

#show link: underline
#show link: text.with(fill: blue)

#title-slide[
  #set par(justify: false)

  = Simulador de partículas determinístico com GPU
]

#slide(title: [Tecnologia])[
  - Rust
  - WGPU

  Segurança, performance, controlo sobre a pipeline de renderização, APIs modernas
]

#slide(title: [Objetivos])[
  - Grande escala
  - Determinismo
  - Colisões

  Que equações usar para mover as particulas?
  E para as colidir?
]

#slide(title: [Escolha do algoritmo de integracao])[
  - *Basic Verlet Integration*
  - Velocity Verlet
  - Euler integration
  - Leapfrog Integration
  - Position Verlet
  - Runge-Kutta Methods (e.g., RK4)
  - Backward Euler
]

#slide(title: [Basic Verlet Integration])[
  // #grid(
  //   columns: (auto, auto),
  //   rows:    (auto, auto),
  //   // gutter: 1em,
  //   [
  //    ],
  //   [
  //   ]
  // )


  ```rs
  struct ParticlePhysics {
    pos: Vec2,
    old_pos: Vec2,
    accel: Vec2,
  }
  ```
  
  Em cada passo, dado um $triangle$t:

  ```rs
  let vel = particle.pos - particle.old_pos;
  particle.old_pos = particle.pos;
  let accel = particle.accel;
  particle.pos += vel + (accel * DELTA_SQUARED);
  particle.accel = Vec2::ZERO;
  ```
]

#slide(title: [Colisoes])[

  // colidimos quando particulas se intersetam
  // dependendo da distancia de intersecao, alteramos a posicao da particula segundo o eixo da intersecao
  // e explicar que atualmente temos N * N

  #align(center)[
    #grid(
      columns: (auto, auto),
      rows:    (auto),
      // gutter: 1em,
      [
        #image("img/colliding1.svg", width: 60%)
      ],
      [
        #image("img/colliding2.svg", width: 75%)
      ]
    )
  ]
]

#title-slide[
  = Otimizações
]

#slide(title: [Otimizações])[

  *Atual:*

  - Comparar todas as particulas com todas as outras ($N^2$)
  - Nao paralelizável
  - *10k* particulas: *4FPS*
  
  *Objetivo:*
  - *100k* particulas, *60FPS*
]


#slide(title: [Binning])[

  - Dividir o espaco em grelha
  - Comparar celulas adjacentes

  #figure(
    placement: none,
    grid(
      columns: (auto, auto, auto),
      rows:    (auto, auto, auto),
      // gutter: 1em,
      [ #image("img/before_bin.svg",   width: 80%) ],
      [ #image("img/bin.svg",   width: 80%) ],
      [ #image("img/after_bin.svg", width: 80%) ],
    ),
  ) <fig:bin>

  - *10k* particulas: *300FPS*
]

#slide(title: [Multithreading])[

  - Cada thread processa colisoes numa parte da grelha
  - Deixa de ser deterministico: sao necessarios 2 passes
  - Colocar particulas na grelha tem de continuar a ser sequencial

  #figure(
    placement: none,
    grid(
      columns: (auto, auto),
      rows:    (auto, auto),
      // gutter: 1em,
      [ #image("img/mt1.svg",   width: 80%) ],
      [ #image("img/mt2.svg", width: 80%) ],
    ),
  ) <fig:mt>

  - *50k* particulas: *50FPS*
]

#slide(title: [Compute shader])[

  - Nao podemos usar a mesma estrutura do multithreading
  - Como manter determinismo?

  #figure(
    placement: none,
    image("img/gpu1.svg", width: 40%)
  ) <fig:gpu1>
]

#slide(title: [Compute shader])[

  - Sao necessarios 9 passes
  - *100k* particulas: *120FPS*

  #let grid_image(path) = {
      box(inset: 2pt, fill: gray, image(path, width: 70%))
  }

  #place(
    dy: -50%,
    dx: 40%,
    figure(
      placement: none,
      grid(
        columns: (20%, 20%, 20%),
        rows:    3,
        gutter: 0em,
        column-gutter: -2em,
        row-gutter: 0.25em,
        // fill: gray,
        // inset: 10pt,
        // [ #grid_image("img/grid0.svg") #grid_image("img/grid3.svg") #grid_image("img/grid6.svg") ],
        // [ #grid_image("img/grid1.svg") #grid_image("img/grid4.svg") #grid_image("img/grid7.svg") ],
        // [ #grid_image("img/grid2.svg") #grid_image("img/grid5.svg") #grid_image("img/grid8.svg") ],
        ..("img/grid0.svg", "img/grid1.svg", "img/grid2.svg", "img/grid3.svg", "img/grid4.svg", "img/grid5.svg", "img/grid6.svg", "img/grid7.svg", "img/grid8.svg").map(grid_image)
      ),
    )
  )
]

#title-slide[
  = Renderização de imagens
]

#slide(title: [Renderização de imagens])[

  - Determinismo permite que particulas acabem todas na mesma posicao

  #align(center,
    grid(
        columns: 5,
        rows: 1,
        gutter: 1em,
        image("img/av.png", width: 100%),
        text(weight: 1000, size: 30pt, "→"),
        image("img/av_resized.png", width: 100%),
        text(weight: 1000, size: 30pt, "→"),
        image("img/av_particles.png", width: 100%),
    )
  )
]
