# LDC v.0.5.0

**Laplace Demon Code**

## Cos'è

LDC è un linguaggio di programmazione per sistemi IoT ed embedded che sto sviluppando come progetto personale.

L'ho iniziato con l'obiettivo di creare un'alternativa a C e MicroPython più semplice da usare e pensata per microcontrollori con poche risorse.

## Obiettivi

* Ridurre l'utilizzo della RAM tramite il chunking automatico
* Semplificare la comunicazione tra più microcontrollori
* Separare la logica dalle regole di sicurezza
* Supportare array fissi e dinamici con capacità massima
* Sfruttare hardware come Teensy 4.x e Arduino Uno

## Stato

* Parser, AST e analisi semantica
* Interprete
* Generazione di codice C
* Array fissi e dinamici (`push`, `pop`, `length`)
* Cicli `for`, `while` e `repeat ... until`, con `break` e `continue`
* `struct`, `enum`, `switch` e `include()` di file sorgente

## Comandi `ldc`

| Comando | Azione |
|---|---|
| `ldc file.spln` | Genera il C in `ldc-output` accanto al sorgente |
| `ldc --run file.spln` | Esegue il programma nell'interprete |
| `ldc --emit-c file.spln` | Scrive il C su stdout |
| `ldc --build file.spln` | Genera il C e compila l'eseguibile |
| `ldc --exec file.spln` | Genera, compila ed esegue |

## Roadmap

- [x] Parser, AST e interprete
- [x] Generazione C
- [x] Array fissi e dinamici
- [x] Cicli e controllo del flusso
- [x] Struct, enum e moduli sorgente con `include()`
- [ ] Import e risoluzione versionata di block/librerie
- [ ] Libreria standard e backend embedded

## Esempio: array e cicli

```spln
fn main {
    let.mut.array[? max 8].unsigned.16bit misure = [];
    push(misure, 10);
    push(misure, 12);

    let.mut.32bit somma = 0;
    for (i in 0..length(misure)) {
        somma = somma + misure[i];
    }
    display("Somma:", somma);
}
```

L'esempio completo, con entrambi i tipi di array e tutti i cicli, è in `Interprete/examples/array_cicli.spln`. Puoi provarlo con l'interprete:

```sh
ldc --run "Interprete/examples/array_cicli.spln"
```

Per generare e compilare il C:

```sh
ldc --build "Interprete/examples/array_cicli.spln"
```

LDC è un progetto sperimentale e la sintassi può cambiare.

## Struct, enum e moduli

Il compilatore supporta struct con campi tipizzati, `Nome::default()`, enum con payload e `switch` con `default`. Gli include sono risolti rispetto alla cartella del progetto; variabili e funzioni incluse si usano con il namespace automatico ricavato dal nome del file.

```spln
struct Lettura { valore: let.mut.32bit }
enum Stato { Valido(32bit), Vuoto }

fn main {
    include("sensori.spln");
    let.mut.Lettura dato = Lettura { valore: sensori.temperatura };
    display(dato.valore);
}
```

Esempi eseguibili:

```sh
ldc --exec "Interprete/examples/struct_enum.spln"
ldc --run "Interprete/examples/moduli.spln"
```

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

Per generare `ldc-output/main.c` e compilarlo in `ldc-output/main` (su Windows `main.exe`):

```sh
ldc --build "Interprete/examples/funzioni.spln"
```

La CLI cerca automaticamente `clang`, `gcc` e `cc` su Linux/macOS/BSD, oppure `clang`, `gcc` e `cl.exe` su Windows. Se non ne trova uno, indica di installare un compilatore C e renderlo disponibile nel `PATH`.

Per compilare e avviare subito il programma, inoltrando direttamente al terminale il suo output:

```sh
ldc --exec "Interprete/examples/funzioni.spln"
```

### Aggiornare `ldc` dopo una modifica al progetto

Dopo aver aggiornato o modificato il codice Rust di LDC, reinstalla il comando:

```sh
cargo install --path Interprete --force
```

LDC è sperimentale è un progetto in corso la sintassi potrebbe cambiare
