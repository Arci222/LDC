# LDC

**Laplace Demon Code**

## Cos'è

LDC è un linguaggio di programmazione per sistemi IoT ed embedded che sto sviluppando come progetto personale.

L'ho iniziato con l'obiettivo di creare un'alternativa a C e MicroPython più semplice da usare e pensata per microcontrollori con poche risorse.

## Obiettivi

* Ridurre l'utilizzo della RAM tramite il chunking automatico
* Semplificare la comunicazione tra più microcontrollori
* Separare la logica dalle regole di sicurezza
* Introdurre nuovi costrutti per i cicli
* Sfruttare al massimo hardware come Teensy 4.x e Arduino Uno

## Stato

- Lexer
- Parser
- AST
- Analisi semantica
- Interprete
- Backend C (generazione di codice C)
- Linguaggio in sviluppo

LDC è un progetto sperimentale e la sintassi può cambiare.

## Esempio

```text
fn somma(a: 32bit, b: 32bit) -> 32bit {
    return a + b;
}

fn main {
    let risultato = somma(10, 32);
    display(risultato);
}
```

## Installazione

Installa Rust tramite [rustup](https://rustup.rs/), seguendo la documentazione ufficiale. Su Windows, per compilare C con `cl.exe`, installa anche Visual Studio Build Tools con “Desktop development with C++”.

Dalla cartella principale del repository:

```sh
cargo install --path Interprete
```

Se `ldc` non viene trovato, aggiungi `~/.cargo/bin` al `PATH` su Linux, macOS e BSD:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

Su Windows Rust aggiunge normalmente `%USERPROFILE%\.cargo\bin` al `PATH`; riapri PowerShell dopo l’installazione.

## Comandi

| Comando | Effetto |
|---|---|
| `ldc file.spln` | Genera `ldc-output/file.c` accanto al sorgente. |
| `ldc --emit-c file.spln` | Scrive il C generato su stdout. |
| `ldc --run file.spln` | Esegue il programma tramite interprete. |
| `ldc --build file.spln` | Genera `ldc-output/main.c` e compila `main` o `main.exe`. |
| `ldc --exec file.spln` | Compila e avvia il programma. |

I comandi accettano percorsi relativi e assoluti. La CLI cerca `clang`, `gcc` e `cc` su Linux, macOS e BSD; su Windows cerca `clang`, `gcc` e `cl.exe`.

## Roadmap

- [x] Lexer
- [x] Parser
- [x] AST
- [x] Analisi semantica
- [x] Interprete
- [x] Backend C
- [ ] Cicli
- [ ] Array
- [ ] Struct
- [ ] Enum
- [ ] Moduli
- [ ] Libreria standard
- [ ] Backend LLVM
