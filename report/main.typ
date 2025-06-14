#let todo = text.with(fill: red, weight: "bold")

#todo[capa]

= Introducao

#todo[palha sobre o que sao particle systems, para que sao usados, que sao computacionalmente pesados, etc. quisemos usar algoritmos minimamente baseados em fisicas e colicoes]

= Algoritmos de integracao

Para poder simular os movimentos das particulas, precisamos de integrar o mesmo, ou seja, usar passos discretos para simular um movimento que realisticamente seria continuo, baseando-nos nas equacoes de movimento.

O grupo investigou algumas alternativas, tais como:

#todo[temos de explicar o que eles sao??]

- Basic Verlet Integration
- Velocity Verlet
- Euler integration
- Leapfrog Integration
- Position Verlet
- Runge-Kutta Methods (e.g., RK4)
- Backward Euler

Tinhamos como prioridade obter boa performance mas tambem estabilidade razoavel da simulacao, pelo que optamos por usar Basic Verlet, que nos pareceu um bom compromisso, dadas as comparacoes efetuadas por diversas fontes???????


= Collisoes
// nao fazem parte da verlet integration, sao outra coisa completamente a parte, nao faco puta de ideia do que o que se chama o algoritmo que escolhemos


== Determinismo

Ao usar um $triangle t$ fixo, a simulacao usando Basic Verlet passa a ser deterministica. Esta propriedade pareceu-nos extremamente interessante, visto que o mesmo conjunto de dados de input ira sempre gerar o mesmo output. Assim, por exemplo, poderiamos gerar imagens dando cores as particulas: como elas iriam sempre calhar nos mesmos sitios, as cores iniciais poderiam ser previstas de modo a representarem as cores das imagens.

= Otimizacoes

O algoritmo atual percorre $N^2$ particulas para detetar colisoes, comparando todas as particulas com todas as outras particulas. Isto e muito ineficiente, e podera ser melhorado tirando proveito do facto de que colisoes entre particulas distantes nao tem de ser computadas pois nao irao surtir qualquer efeito.
Para alem disso, nao e paralelizavel, deixando de lado possiveis ganhos de performance. Assim, implementamos algumas otimizacoes:

== Binning

Para aproveitar o facto de colisões so surtirem efeito se ocorrerem entre particulas proximas, podemos usar um algoritmo de binning, dividindo o espaco numa grelha (em bins) e colidindo apenas particulas entre celulas adjacentes. 

Assim, no inicio de cada passo de simulacao, o algoritmo coloca cada particula numa celula da grelha com base na sua posicao. Para calcular colisões, basta, para cada celula, iterar as suas particulas, calculando colisoes apenas com as particulas dentro da propria celula e nas celulas que a rodeiam.

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
  caption: [Particulas com que a particula vermelha precisa de ser comparada.
  
  Esquerda: sem binning. Meio: particulas em celulas adjacentes. Direita: com binning.]
) <fig:bin>

#todo[numeros a mostrar o quanto isto melhorou]

== Multithreading

Visto o espaco agora estar organizado numa grelha, e possivel implementar multithreading neste algoritmo, atribuindo uma parte da grelha a cada thread.

No entanto, diferentes threads podem aceder a celulas adjacentes, provocando data races nas colisoes entre as particulas das mesmas. Estas data races tornam o algoritmo nao deterministico, o que vai contra o nosso objetivo.

Para voltar a tornar a simulacao deterministica, decidimos fazer 2 passes de simulacao distintos, procurando evitar que threads diferentes possam estar a processar a mesma celula:

#figure(
  placement: none,
  grid(
    columns: (auto, auto),
    rows:    (auto, auto),
    // gutter: 1em,
    [ #image("img/mt1.svg",   width: 80%) ],
    [ #image("img/mt2.svg", width: 80%) ],
  ),
  caption: [Os espacos cinzento e azul estao a ser processados por duas threads distintas. Sao feitos dois passes (esquerda e direita), garantindo que ao processar celulas azuis e cinzentas nunca ha celulas adjacentes a serem processadas em simultaneo]
) <fig:mt>


== Padding

Ao calcular quais as celulas que estao em redor de uma dada celula, existem varias posicoes na grelha que precisam de cuidados especiais, o que complica o codigo, como as celulas que se situam nas edges da grelha, e especialmente nos cantos. Seria necessario ter casos especiais em que determinavamos se se trata de uma destas situacoes, e impedir, por exemplo, a comparacao com celulas acima devido a estas nao existirem.

Assim, decidimos acrescentar uma camada de padding, ou seja, bins que nunca contem qualquer particula nem sao processadas, de forma a uniformizar o codigo e remover condicoes desnecessarias do mesmo.

Esta otimizacao torna-se especialmente relevante para evitar thread divergence, que sera util para poder executar a detecao de colisoes na GPU no futuro.

= Compute shaders

Com o objetivo de aumentar ainda mais o número de partículas na nossa simulação, concluímos que apenas com a capacidade computacional de uma GPU seria possível.

== Colisao basica

Numa primeira fase, decidimos implementar novamente o algoritmo ineficiente que compara todas as particulas com todas as outras particulas.

Apesar dos problemas obvios deste algoritmo, ainda assim tivemos enormes ganhos de performance .......

#todo[numeros a mostrar o quanto melhorou]

== Binning

A solucao ideal, usando binning, traz uma nova dimensao aos problemas encontrados na implementacao de multithreading. Tendo uma arquitetura paralela, evitar race conditions torna-se desafiante, mas necessario.

Infelizmente, para manter o determinismo, o passo de gerar as bins tera sempre de ser feito no cpu, ou pelo menos sorted no mesmo, para garantir que as particulas sao sempre processadas na mesma ordem, pelo que nao conseguimos evitar perdas vindas da latencia de transferencia de dados GPU$<->$CPU.

Nas colisoes, a solucao de usar varios passes diferentes, na solucao de multithreading, tera de ser adaptada a arquitetura da GPU: nao e razoavel que uma thread processe uma zona inteira do espaco da grelha, mas sim uma unica celula. Assim, para cada celula, teremos de garantir que existe um espaco de 2 celulas livre tanto na vertical como na horizontal:

#figure(
  placement: none,
  image("img/gpu1.svg", width: 40%)
  // caption: [Os espacos cinzento e azul estao a ser processados por duas threads distintas. Sao feitos dois passes (esquerda e direita), garantindo que ao processar celulas azuis e cinzentas nunca ha celulas adjacentes a serem processadas em simultaneo]
) <fig:gpu1>

Enquanto na solucao com multithreading seria suficiente 2 passes de simulacao, aqui serao necessarios 9 para que todas as celulas sejam devidamente processadas:

#figure(
  placement: none,
  grid(
    columns: (auto, auto, auto),
    rows:    (auto, auto, auto),
    // gutter: 1em,
    [ #image("img/grid0.svg", width: 95%) #image("img/grid3.svg", width: 95%) #image("img/grid6.svg", width: 95%) ],
    [ #image("img/grid1.svg", width: 95%) #image("img/grid4.svg", width: 95%) #image("img/grid7.svg", width: 95%) ],
    [ #image("img/grid2.svg", width: 95%) #image("img/grid5.svg", width: 95%) #image("img/grid8.svg", width: 95%) ],
  ),
  caption: [Os 9 passes necessarios para calcular as colisoes de todas as celulas, evitando data races]
) <fig:mt>




#todo[substeps]

#todo[vert e frag shaders, explicar como os dados ja estao na gpu entao temos boa perf]

#todo[como imagens sao carregadas]

#todo[mostrar um resultado final, sq um video para o youtube]

#todo[otimizacoes futuras: algoritmo do dudu, e como passamos a tirar partido do facto de existirem workgroups]

#todo[reduce para contar o numero de particulas?]

