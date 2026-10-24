# Esempi per l'interprete LDC

Questi file usano la sintassi attualmente implementata in `Interprete/src/parser.lalrpop`: blocchi `env`, `capabilities`, `identity`, `fn main`; variabili scalari, funzioni scalari con parametri e `return`, chiamate, `display`, espressioni, `if` e chunk `#nome`. Il manuale LDC descrive anche costrutti che questo prototipo non implementa ancora.

Dalla radice del repository, esegui un esempio così:

```sh
cargo run --manifest-path Interprete/Cargo.toml -- esempi/01_variabili.spln
```

L'esempio con le funzioni è in `Interprete/examples/funzioni.spln`:

```sh
cargo run --manifest-path Interprete/Cargo.toml -- Interprete/examples/funzioni.spln
```

Per visualizzare il C generato dallo stesso esempio:

```sh
cargo run --manifest-path Interprete/Cargo.toml -- --emit-c Interprete/examples/funzioni.spln
```

Per eseguire tutti gli esempi in sequenza:

```sh
for file in esempi/*.spln; do
  echo "== $file =="
  cargo run --quiet --manifest-path Interprete/Cargo.toml -- "$file" || break
done
```
