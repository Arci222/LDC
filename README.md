# LDC

**Laplace Demon code**

## Cos'è

LDC è un linguaggio di programmazione per sistemi IoT ed embedded che sto sviluppando come progetto personale.

L'ho iniziato a 14 anni con l'obiettivo di creare un'alternativa a C e MicroPython più semplice da usare e pensata per microcontrollori con poche risorse.

## Obiettivi

* Ridurre l'utilizzo della RAM tramite il chunking automatico
* Semplificare la comunicazione tra più microcontrollori
* Separare la logica dalle regole di sicurezza
* Introdurre nuovi costrutti per i cicli
* Sfruttare al massimo hardware come teensy 4.0 e e arduino UNO

## Stato

* Parser
* AST
* Interprete
* Linguaggio in sviluppo
* codegen C

<<<<<<< HEAD
LDC è un progetto sperimentale e la sintassi può cambiare.
=======
SNACK è un progetto sperimentale e la sintassi può cambiare.

## Comando `ldc`

Con Rust installato, installa il comando dalla cartella principale del repository:

```sh
cargo install --path Interprete
```

Passagli il percorso del sorgente; il C viene scritto in `ldc-output/<nome>.c` accanto al file `.spln` e il comando stampa il percorso completo:

```sh
ldc /percorso/assoluto/al/file.spln
```

Il percorso d'uscita non usa `/tmp`, quindi funziona anche su Windows e macOS. Se il percorso contiene spazi, racchiudilo tra virgolette. Passa un file per volta. `ldc --run <file.spln>` interpreta il programma; `ldc --emit-c <file.spln>` stampa il C sul terminale.
>>>>>>> f66f04c (comando LDC percorso)
