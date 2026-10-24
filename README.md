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

LDC è un progetto sperimentale e la sintassi può cambiare.

## Installazione e primo utilizzo

LDC è scritto in Rust. Dopo aver scaricato o clonato questa cartella da GitHub, apri un terminale nella cartella principale del progetto e installa Rust se non è già disponibile.

### Linux, macOS e BSD

Il metodo consigliato è `rustup`:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Chiudi e riapri il terminale, oppure carica subito la configurazione della shell:

```sh
. "$HOME/.cargo/env"
```

Su macOS può servire prima installare gli strumenti C di Xcode:

```sh
xcode-select --install
```

Su alcune distribuzioni BSD, `rustup` potrebbe non essere disponibile nei repository o non supportare la piattaforma. Installa allora i pacchetti `rust` e `cargo` con il gestore della tua distribuzione, poi continua con i comandi seguenti.

### Windows

Installa Rust tramite [rustup](https://rustup.rs/), scegliendo la toolchain predefinita. Se il programma di installazione lo richiede, installa anche **Visual Studio Build Tools** con il carico di lavoro “Desktop development with C++”.

Apri PowerShell nella cartella del progetto per eseguire i comandi seguenti.

### Installare il comando `ldc`

Da Linux, macOS, BSD o Windows, nella cartella principale del repository esegui:

```sh
cargo install --path Interprete
```

Questo installa il comando `ldc` per il tuo utente.

Se il terminale risponde che `ldc` non è stato trovato, aggiungi la cartella dei binari Rust al `PATH`.

Su Linux, macOS e BSD con Bash o Zsh:

```sh
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

Se usi Zsh, sostituisci `~/.bashrc` con `~/.zshrc`. Per usarlo solo nella sessione corrente:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

Su Windows il programma di installazione di Rust aggiunge normalmente `%USERPROFILE%\.cargo\bin` al `PATH`. Chiudi e riapri PowerShell; se serve aggiungilo manualmente nelle variabili d’ambiente dell’utente e riapri il terminale.

Verifica l’installazione:

```sh
ldc --help
```

### Compilare un file LDC

Passa a `ldc` il percorso del file `.spln`. Il percorso può essere relativo o assoluto; racchiudilo tra virgolette se contiene spazi.

```sh
ldc "Interprete/examples/funzioni.spln"
```

Il comando genera C in una cartella accessibile accanto al sorgente:

```text
Interprete/examples/ldc-output/funzioni.c
```

Non usa `/tmp`, quindi questo comportamento è uguale su Linux, macOS, BSD e Windows. Il percorso completo del file creato viene mostrato nel terminale.

Per eseguire il sorgente con l’interprete senza generare C:

```sh
ldc --run "Interprete/examples/funzioni.spln"
```

Per stampare il C generato direttamente nel terminale:

```sh
ldc --emit-c "Interprete/examples/funzioni.spln"
```

### Aggiornare `ldc` dopo una modifica al progetto

Dopo aver aggiornato o modificato il codice Rust di LDC, reinstalla il comando:

```sh
cargo install --path Interprete --force
```
