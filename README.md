# cruzante

Un programa chico en Rust que intenta pasar un lobo, una cabra y un col al otro lado del río, y **escribe un hijo que se queda con los viajes que todavía eran legales**.

No es un modelo de lenguaje y no se copia solo. Le señalás una carpeta; sólo escribe ahí.

Es hermano de los constructores que se copian, buscan, prueban o persiguen un mundo que se mueve. Esos quedan como están. Este usa el mismo molde (genoma embebido, un hijo, destino explícito) sobre un acertijo congelado: el bote lleva al barquero y a un pasajero. El lobo no puede quedar con la cabra. La cabra no puede quedar con el col.

Lo que evoluciona es un **plan**: una lista de cargas (`lobo`, `cabra`, `col`, `nada`). El barquero rema siempre. Si un viaje es ilegal o alguien se come a alguien, ese es el parto. El hijo hereda el prefijo que sobrevivió y reescribe desde el fracaso. No baraja los viajes que ya servían.

```
padre     plan  lobo                         la cabra se come el col
  │ no guarda nada (el primer viaje mató)
  ▼
          plan  cabra                        legal hasta acá
  │ guarda ese viaje, cambia el siguiente
  ▼
hijo      plan  cabra nada lobo cabra col nada cabra
```

![Dos orillas: se descarta el viaje que mató, se guarda el prefijo legal](cell.svg)

Míralo en la terminal. Orilla izquierda, agua, orilla derecha. El barquero es `@`. `dish` usa por defecto la seed 7.

```bash
cargo run -- dish
```

## Cómo correrlo

Hace falta [Rust](https://rustup.rs/).

```bash
cargo build --release
./target/release/cruzante identity
./target/release/cruzante evolve --steps 80 --spawn ./hijo --build
./hijo/target/debug/cruzante identity
```

`identity` imprime generación, plan y quién está en cada orilla.  
`evolve --spawn ./hijo --build` busca, conserva prefijos legales, y escribe un crate hijo con el plan ganador (o el mejor).

## Comandos

```
cruzante identity              generación, linaje, plan, orillas
cruzante eval [plan]           simula este plan, o el actual
cruzante evolve                busca; los mutantes copian el prefijo legal
                 --steps N      pasos de búsqueda (default 80)
                 --lambda L     mutantes por paso (default 20)
                 --seed S       rng reproducible
                 --spawn <dir>  hijo con el campeón
                 --build        compila a ese hijo
                 --force        pisa un hijo anterior
                 --write        pisa src/main.rs de este proyecto
cruzante dish                  anima las dos orillas
                 --steps N      pasos de búsqueda (default 80)
                 --lambda L     mutantes por paso (default 20)
                 --seed S       default 7
                 --delay MS     ms por cuadro (default 80)
cruzante spawn <dir>           copia el genoma actual (sin buscar)
cruzante genome                imprime las fuentes embebidas
```

`--spawn` deja este programa en paz y escribe un hijo elegido.  
`--write` edita el `src/main.rs` de este proyecto; compilá de nuevo para que el binario nazca con el plan nuevo.

## Cómo funciona

El plan vive en una constante de `src/main.rs`.

1. Parsea el plan actual a tokens de carga.
2. Simula desde la orilla izquierda. Para en el primer viaje ilegal o en la primera comida.
3. Cada mutante **copia el prefijo legal** y sólo cambia lo que viene después.
4. Menor puntaje es mejor: un cruce completo le gana a un prefijo largo; un plan ganador más corto le gana a uno más largo.
5. Con `--spawn`, escribe un proyecto Cargo completo cuya fuente contiene ese plan.

No hay un algoritmo de caminos adentro. El río no cambia. Esto no es un certificado de Gödel: cualquiera puede volver a correr `eval` sobre el plan.

## Seguridad

- Un hijo por corrida. No hay bucles en segundo plano ni red.
- No escribe sobre el directorio home, `/`, `/usr`, `/etc`, ni el directorio en el que estás parado.
- `--force` sólo borra una carpeta que ya parece un proyecto `cruzante`.

## Relacionados

El molde (sin reabrir):  
[replicante](https://github.com/PascualMacana/replicante) se copia.  
[mejorante](https://github.com/PascualMacana/mejorante) busca contra una curva congelada.  
[demostrante](https://github.com/PascualMacana/demostrante) sólo escribe una mejora afirmada si hay una prueba verificable.  
[reinante](https://github.com/PascualMacana/reinante) sigue buscando porque el objetivo mismo se mueve.
