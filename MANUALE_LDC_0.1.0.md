# LDC 0.1.0 — Manuale

> **Note di revisione — v1:** `reset;` non più elencato tra gli eventi che mantengono il valore dei top-level (§I.2 — riporta invece allo stato di dichiarazione, dettagli in §II.8); tabella crash/reset in §II.8 corretta di conseguenza; bit-size aggiunto al campo `label` nelle struct d'esempio (§I.5, §II.13); chiarito il pull esplicito via path a chunk terminato (§I.15).
>
> **v2 — 24 punti aperti risolti:** comportamento completo di `reset;` (§II.8) · tipo di `throw` (§II.7) · `checkpoint;` nei listener (§II.8) · scope dei listener sui chunk (§II.0) · scrittura tramite array originale durante un `take` (§I.6) · motivazione del crash su `push` pieno (§I.6) · perimetro esatto dell'overflow (§I.4) · conteggio caratteri Unicode (§I.9) · stampa di struct con `display()` (§I.9) · natura di `TypeIdentity` (§I.10) · `pointer()` a indirizzo calcolato (§II.11) · assembly inline (§II.11) · bridge offline/pacchetti persi (§II.5) · timing di scrittura di `rule{}` (§II.2) · wear leveling EEPROM (§II.2) · ritorno di closure da funzioni (§I.13) · copia vs riferimento in `for` (§I.12) · watchdog nei loop senza yield (§I.12) · allineamento struct nell'ABI (§III.4) · conflitto tra tipi opachi omonimi (§III.0) · costanti hardware da blocchi (§I.11) · criterio compile-time di `read_file()` (§II.4) · motivazione del `default` obbligatorio (§I.7) · const shallow vs deep (§I.4).
>
> **v3 — 2 punti risolti:** formula `MAX_STRING_LENGTH` vincolata a una frazione ≤1 di `CHUNK_SIZE` (non più un moltiplicatore libero che poteva superare `env.ram`), con rigetto compile-time di `chunk_ram_ratio`/`max_string_multiplier` > 1 (§II.6) · `accessibility{}` rimosso da `main.ldc`: ora è un profilo utente in `~/.laplace/accessibility.toml`, legato all'installazione di Laplace e non al singolo progetto, non versionato, non estratto in `ldc.toml` (§II.14, ex capitolo 35 a sé stante).
>
> Notazione riferimenti: `§I.N` = Parte I sezione N, `§II.N` = Parte II sezione N, `§III.N` = Parte III sezione N. Senza prefisso = stessa parte del testo che lo contiene.

## Indice

**Parte I — Fondamenta:** 0. Introduzione · 1. Sintassi Base · 2. Struttura del Programma · 3. Autogenerazione — `ldc.toml` · 4. Variabili e Tipi · 5. Tipi Composti · 6. Array Dinamici · 7. Enum e Switch · 8. Operatori · 9. Funzioni Built-in · 10. Introspezione · 11. Variabili Built-in · 12. Controllo di Flusso · 13. Funzioni · 14. Riferimenti · 15. Gerarchia dei Chunk

**Parte II — Runtime:** 0. Listener · 1. On-Event · 2. Rule{} e Sicurezza · 3. Comandi Hardware · 4. File I/O e Include · 5. Multi-Ambiente e Bridge · 6. Gestione Memoria · 7. Gestione Errori · 8. Checkpoint e Reset · 9. Ottimizzazioni · 10. Tooling e Debug · 11. Riferimenti e Puntatori Grezzi · 12. Linked List, Grafi e Alberi · 13. Esempio Completo · 14. Accessibilità

**Parte III — Ecosistema:** 0. Blocchi · 1. Hash, Firma e Trust · 2. Registry Laplace · 3. Autogenerazione · 4. ABI e Binari Stabili

---

# Parte I — Fondamenta

## 0. Introduzione

LDC (**Laplace Demon Code**) è un linguaggio per sistemi embedded, compilato in C. Ogni decisione punta a una cosa: **sapere a compile-time quanta memoria serve**, zero sorprese a runtime. Estensione file: **`.ldc`**.

**Modello a 2 file totali per progetto:**
- **`main.ldc`** — scritto dall'utente. Contiene TUTTO: hardware, dipendenze, sicurezza, logica.
- **`ldc.toml`** — autogenerato da Laplace. Contiene hash, lock, risoluzioni, ABI. Mai scritto a mano.

(L'unica eccezione è `~/.laplace/accessibility.toml`, §II.14 — vive fuori dal progetto per scelta, non fa parte di questo conteggio.)

Il developer scrive **solo** `main.ldc`. Laplace genera `ldc.toml` ad ogni build. Se non esiste, viene creato con default sicuri; se esiste, viene aggiornato — tranne le sezioni immutabili (§II.2 / §III.3), preservate.

## 1. Sintassi Base

**Encoding:** UTF-8. Case-sensitive. Newline LF o CRLF.

**Commenti:** `//` su una riga, `/* */` su più righe.

**Nomi:** iniziano con lettera Unicode (categoria `L*`) o `_`, poi lettere `L*`, cifre decimali (categoria `Nd`), underscore. Niente spazi né trattini. Nessuna normalizzazione Unicode: il compilatore confronta i byte così come sono, quindi `café` (NFC) e `café` composto con combining mark (NFD) sono due identificatori distinti — si raccomanda che editor/linter normalizzino in NFC, non è imposto dal compilatore.
```text
let.mut.32bit contatore_1 = 0;   // valido
let.mut.32bit _privata = 0;      // valido
let.mut.32bit 1nome = 0;         // NON valido
```

**Punteggiatura:**

| Simbolo | Uso |
|---|---|
| `;` | fine istruzione |
| `,` | separatore in liste/parametri |
| `{ }` | delimitano blocchi |
| `/` | separatore di path nell'albero dei chunk (solo in un contesto di path, es. `fn_main/#trip`; altrove è l'operatore di divisione, §8) |

`checkpoint;` e `reset;` non sono punteggiatura: sono istruzioni runtime a tutti gli effetti — vedi §II.8.

**Literal:** `42`, `-3.14`, `"testo"`, `true`, `false`. Non esiste un valore nullo **leggibile**: ogni variabile, nel momento in cui viene letta, ha sempre un valore concreto (vedi l'eccezione controllata sotto, che riguarda solo la fase tra dichiarazione e assegnazione).

**Eccezione controllata:** `let x;` senza valore è permesso, ma il compilatore vieta di **leggere** `x` finché non dimostra, su ogni percorso, che è assegnata (definite assignment analysis, zero costo a runtime).
```text
let.mut.32bit result;
if (sensor_ready) { result = read_analog(A0); } else { result = 0; }
display(result);   // OK

let.mut.32bit result;
if (sensor_ready) { result = read_analog(A0); }
display(result);   // ERRORE: ramo else non assegna
```

## 2. Struttura del Programma

Un file `.ldc` ha blocchi in ordine fisso, poi `fn main`. **Obbligatori: solo `env` e `fn main`.** Tutti gli altri blocchi (`dependencies`, `capabilities`, `security`) sono opzionali, con default dichiarati sezione per sezione. **TUTTO è dentro il `.ldc`** — nessun TOML separato.

```text
env {
    target: "arduino_uno";
    ram: 2048;                                    // byte
    hardware: [GPIO_13, GPIO_17, ADC_A0, ADC_A1];  // whitelist di intento, non uso obbligatorio
    sensors: [DHT22_temp, DHT22_humidity];
    watchdog_timeout: 5000;                        // ms
}

dependencies {
    os = "1.0.0";
    network = ">=2.0, <3.0";
    sensors_lib = "1.2.0";
}

capabilities {
    import: "sensors_lib";
    uses: pwm;
    uses: sensors_analog;
}

security {
    max_hardware_speed = 30;
    max_hardware_temp = 85;
    can_modify_max_speed = false;
    allow_unsafe_blocks = false;
    unsafe_allowed_modules = ["./hardware_driver.ldc"];
    chunk_ram_ratio = 0.6;
    max_recursion_depth = 32;
}

identity {
    name: "scooter_controller";
    let.mut.32bit total_distance = 0;
    fn compute_speed(d: 32bit, t: 32bit) -> 32bit { return d / t; }
    rule { let.mut.unsigned.8bit max_speed = 25; }
}

fn main { /* logica, chunk annidati */ }
```

- **`env`** — hardware, RAM, target, watchdog.
- **`dependencies`** — opzionale. Se assente, si risolvono automaticamente dall'ultima versione stabile.
- **`capabilities`** — permessi hardware e import.
- **`security`** — opzionale. Se assente, default sicuri. Se presente, **preservato** in `ldc.toml` e immutabile a runtime.
- **`identity`** — nome, globali, `rule{}`.
- **`fn main`** — ingresso. Niente `->`, non ha tipo di ritorno.

**Convenzione unità di misura**, valida in tutto il manuale salvo indicazione diversa nel nome del campo: durate e timeout in **millisecondi**, temperature in **gradi Celsius**. Campi con unità diversa la portano nel nome (es. `_hz`, `_v`).

**`env.hardware`** dichiara i pin che il programma *intende* usare — è una whitelist di intento per validazione statica, non un vincolo di uso obbligatorio. Usare in `set_pin_mode`/`write_pin`/ecc. un pin non elencato in `env.hardware`: warning a compile-time, non errore (il pin esiste comunque sul target).

**Persistenza:** `let` top-level di `fn main` o in `identity` (fuori da `rule{}`) persiste tra chunk/checkpoint/riavvio. `reset;` invece li riporta allo stato di dichiarazione, non a un valore mantenuto (§II.8). `let` dentro chunk è locale.

**Nota sui campi di `env`/`dependencies`/`capabilities`/`security`/`identity`:** questo manuale li specifica tramite esempi rappresentativi, non come schema esaustivo campo-per-campo — quello vivrà nella futura grammatica formale (EBNF) del linguaggio, non duplicato qui in prosa per ognuno dei sei blocchi.

## 3. Autogenerazione — `ldc.toml`

`ldc.toml` è generato da Laplace. **L'utente non lo scrive mai.** Contiene: metadata generati (chi, quando, hash di `main.ldc`); metadata di progetto estratti da `main.ldc`; blocchi risolti (hash precisi, store path); configurazione di esecuzione (default sicuri); `[security_rule]`, estratto dal blocco `security { }` di `main.ldc` e preservato ad ogni rigenerazione.

Esempio generato:
```toml
# ldc.toml — AUTOGENERATO DA LAPLACE
# NON MODIFICARE A MANO

[meta]
generated_by = "laplace 0.1.0"
generated_at = "2026-09-11T15:00:00Z"
source = "main.ldc"
source_hash = "blake3:abc123..."

[project]
name = "scooter_controller"
version = "0.1.0"

[blocks]
os = { version = "1.0.0", hash = "blake3:aaa111...", path = "~/.laplace/store/blake3-aaa111-os-1.0.0/" }

[integrity]
hash_algorithm = "blake3"
require_signature = false
trust_model = "tofu"

[registry]
url = "https://blocks.ldc.dev"
priority = "first"

[runtime]
seccomp_enabled = true
seccomp_default = "deny"

[crypto]
encryption_default = "e2e_ecc"

[log]
level = "info"

[security_rule]
max_hardware_speed = 30
max_hardware_temp = 85
# ... preservato da main.ldc
```

**Dove si trova:** accanto a `main.ldc`. Rigenerato ad ogni build. Se cancellato, ricreato con default.

**Immutabilità di `[security_rule]`:** protetta da password OS, nessun codice/`listener` può modificarla a runtime, ogni scrittura loggata. Preservata ad ogni rigenerazione. Modificabile solo cambiando il blocco `security { }` in `main.ldc` (richiede `laplace security edit` con password). "Password OS" significa che Laplace **non ha un proprio sistema di password**: delega al meccanismo di autenticazione nativo del sistema operativo (`sudo`/PAM su Linux, Touch ID/Windows Hello dove disponibile) — nessuna credenziale propria di Laplace da gestire o poter dimenticare.

## 4. Variabili e Tipi

**Forma semplice:** `let x = 10;` (mutabile, dedotto, default `i32`).

**Forma esplicita:**
```text
let.<mut|const>[.contenitore].[unsigned].<n>bit[.ptr][.float|.string|.bool] nome = valore;
```
Ordine fisso: `mut/const → contenitore → unsigned → Nbit → ptr → float/string/bool`. `.ptr` mutuamente esclusivo con `float/string/bool`. `.bool` facoltativo. `Nbit` obbligatorio per numerici e stringhe — **non** per `bool` (dimensione fissa, 1 byte) né per tipi con nome proprio (`struct`, `tuple`, `enum`: la dimensione è quella dei loro campi, non un tag a parte). `unsigned` è **opzionale**: se omesso, il tipo è **signed** (complemento a due) — `let.mut.32bit x` equivale a `let.mut.signed.32bit x`.

```text
let.mut.unsigned.8bit age = 30;
let.const.32bit.float pi = 3.14159;
let.const.16bit.string label = "temp_A0";
let.mut.bool active = true;
```

**Peso stringhe:** categoria, non bit.
| Tag | Buffer massimo |
|---|---|
| `8bit.string` | 255 byte |
| `16bit.string` | 65.535 byte |
| `32bit.string` | 4 GB |

Le stringhe contengono sempre testo codificato in **UTF-8**; in 0.1 non sono supportati encoding alternativi. I tipi interi disponibili in 0.1 sono solo `8bit`, `16bit` e `32bit`, ciascuno signed o unsigned. Non esiste un tipo intero `64bit`.

**Float:** `32bit.float` è IEEE 754 binary32 (singola precisione). Non esiste `64bit.float` in 0.1 — su MCU 8/16 bit il costo di una FPU software per double precision non è giustificato dai casi d'uso tipici embedded.

**`void`:** non è un tipo dichiarabile per una variabile — esiste solo come tipo di ritorno di funzione (`-> void`, §13), a marcare l'assenza di valore restituito.

**`ptr`:** `Nbit.ptr` non è la dimensione del dato puntato, è la dimensione dell'**indirizzo** sul target corrente — su la maggior parte dei target è `32bit.ptr`, ma su AVR (indirizzi a 16 bit) il compilatore accetta e genera `16bit.ptr`; usare `32bit.ptr` su un target con indirizzi a 16 bit: errore di compilazione (l'indirizzo non ci entra).

**`const` su struct:** blocca riassegnazione della variabile, non mutabilità dei campi.
```text
let.const.SensorData sensor = {...};
sensor.readings[0] = 42;   // OK se campo mut
// sensor.label = "x";     // ERRORE se campo const
```
Questa è una **const shallow**, non deep: blocca solo la riassegnazione del binding esterno. La mutabilità di ogni campo, a qualunque profondità di annidamento (struct dentro struct, array dentro struct, ecc.), dipende **esclusivamente** dal tag `mut`/`const` dichiarato su quel campo specifico — indipendentemente dal fatto che la variabile contenitore più esterna sia `let.mut` o `let.const`.

**Overflow:** crash + ripartenza dall'ultimo `checkpoint`. Vale per ogni scrittura in una variabile a bit fissi che eccede il suo range: operatori aritmetici, compound assignment (`+= -= *= /= %= **= ++ --`), shift (`<< >>`) quando il risultato non entra nel tipo, cast (`as`), e aggregatori come `sum()`. `average()` (ritorna float) e `random()` (già vincolato a `[min,max]`) non sono soggetti a questo overflow. La negazione unaria (`-x`) segue la stessa regola: `-(-128) as 8bit` crasha, perché `128` non entra in un `8bit` signed (range `-128..127`).

`as` **non tronca**: crasha se il valore non entra nel tipo di destinazione, coerente con tutto il resto di questa sezione. Per un troncamento esplicito e mai crashante, usare `wrapping_as` (§8) — due operatori distinti per due intenzioni distinte, invece di un `as` che a volte crasha e a volte no.

## 5. Tipi Composti

**`struct`:** campi fissi tipizzati. Niente `private`/`public`.
```text
struct SensorData {
    readings: let.mut.array[10].unsigned.16bit,
    label: let.const.16bit.string,
    average: let.mut.32bit.float
}
```

**`array`:** dimensione fissa, bounds check automatico.
```text
let.mut.array[10].unsigned.16bit temperature = 0;
let.const.array[6].unsigned.8bit pwm_levels = [0, 50, 100, 150, 200, 255];
length(temperature); average(temperature); sum(temperature); min(temperature); max(temperature);
```

**`tuple`:** lunghezza fissa, tipo diverso per slot.
```text
let.const.tuple(32bit.float, 32bit.float, 16bit.string) coordinate = (45.4642, 9.1900, "deposito");
```

**Array di struct:** `Nome::default()` genera N copie.
```text
let.const.array[5].SensorData readings_log = SensorData::default();
```
`default()` popola ricorsivamente ogni campo con lo zero del suo tipo: numerici a `0` (o `0.0` per float), `bool` a `false`, `string` a `""`, `array` con ogni elemento a sua volta a zero, `struct` annidata applicando `default()` ai suoi campi. Vale anche per campi `const`: `default()` è l'unico modo per assegnare un valore iniziale a un campo `const` di un array generato così.

**Matrici:** struct con array dentro, quello struct dentro array esterno.
```text
struct Riga { valori: let.mut.array[5].unsigned.32bit }
let.mut.array[10].Riga matrice;
matrice[3].valori[2] = 100;
```
Un `array` dichiarato senza inizializzatore esplicito (come `matrice` sopra) è **zero-inizializzato automaticamente**, campo per campo, con la stessa regola di `default()` — eccezione dichiarata alla definite-assignment analysis di §I.1: imporla campo per campo su array/struct di grandi dimensioni sarebbe impraticabile. Una variabile scalare (`let.mut.32bit x;`) segue invece la regola normale di §I.1: nessuno zero implicito, lettura vietata finché non è dimostrata l'assegnazione.

**Casi limite dei tipi composti:**
- `array` **richiede sempre** una dimensione: `array[N]` (fissa) o `array[? max N]` (dinamica, §6). Un `array` senza nessuna delle due forme non è sintassi valida.
- `tuple` con un solo elemento (`tuple(32bit)`) è valida ma senza vantaggi su una variabile scalare diretta; `tuple()` vuota non è supportata in 0.1 (nessun caso d'uso: non porta informazione).
- `enum` con **payload multiplo per variante** è supportato: `enum Result { Ok(32bit, string), Err(string) }`, estratto in `switch` con `case Result::Ok(val, msg): ...`.
- `struct` senza campi (`struct Empty {}`) è sintassi valida ma priva di utilità pratica; tutti i campi di uno `struct` sono sempre obbligatori alla creazione di un'istanza — non esistono campi "opzionali": ogni istanza deve fornirli tutti (o passare da `default()`, sopra).

## 6. Array Dinamici

**`array[? max N]`:** dimensione runtime, tetto massimo obbligatorio.
```text
let.mut.array[? max 100].unsigned.16bit temperature = [];
push(temperature, 22);
```

**Comportamento:** `push` su pieno: crash. `pop` su vuoto: crash. `average`/`min`/`max` su vuoto: crash. `sum` su vuoto: 0.

**Perché crash e non `try`/`catch`:** superare `max N` è la violazione di un limite deciso dal developer a compile-time (dimensiona `max N`, o controlla `length(arr) < max` prima di `push`) — non un evento esterno imprevedibile come un file mancante. Stessa categoria di overflow e divisione per zero (§8): errore di runtime non recuperabile via `catch`, gestito solo da `checkpoint`/crash recovery.

**`take(arr, i)` / `take(arr, inizio, fine)`:** riferimento, non copia. `fine` è **esclusivo** (stessa convenzione di `0..N` in `for`, §12): `take(arr, 2, 5)` copre gli indici 2, 3, 4. `take(arr, i, i)` è legale e produce uno slice vuoto. Un solo `take` **mutabile** attivo alla volta; più `take` di **sola lettura** possono coesistere, anche su intervalli sovrapposti fra loro (il conflitto è solo lettura/scrittura o scrittura/scrittura, mai lettura/lettura). `push`/`insert`/`remove`/`clear`/`unlink` rifiutati mentre esiste **qualsiasi** `take` (anche sola lettura) — stesso motivo: potrebbero spostare o invalidare gli elementi a cui il `take` punta.

Scrivere un elemento già esistente tramite l'array originale (es. `array[i] = x`) resta permesso, tranne quando l'indice ricade nell'intervallo coperto da un `take` attivo (mutabile o di sola lettura) — stessa esclusione delle referenze (§14): array originale e `take` non possono leggere/scrivere in conflitto sullo stesso intervallo mentre il `take` è vivo. Indici fuori da quell'intervallo restano liberamente scrivibili.

Esempio concreto:
```text
let slice = take(temperature, 2, 5);   // copre gli indici 2, 3, 4
temperature[7] = 99;                   // OK: fuori dall'intervallo del take
temperature[3] = 99;                   // ERRORE di compilazione: indice 3 è dentro il take attivo
```

## 7. Enum e Switch

**Enum semplice:**
```text
enum ConnectionPolicy { Continue, Suspend }
```

**Enum con dati:**
```text
enum Link { Node(32bit), End }
```
Accesso solo via `switch` con pattern binding:
```text
switch (cursore) {
    case Link::Node(i): display(i); break;
    case Link::End: break;
    default: break;
}
```
`default` sempre obbligatorio — **non** è una svista del compilatore, vedi il motivo sotto.

**Confronto diretto (`==`/`!=`) con una variante senza payload** (es. `cursore != Link::End`) è permesso ed è l'unico caso di accesso a un `enum` fuori da `switch`: confrontare con una variante senza dati non richiede pattern binding, quindi non c'è nulla da estrarre. Confrontare con una variante **con** payload (es. `cursore == Link::Node(1)`) non è permesso: serve `switch` per poter estrarre il valore contenuto.

**Perché `default` è sempre obbligatorio** (anche quando lo `switch` sembra già esaustivo): scelta deliberata per la compatibilità tra versioni. Un `enum` può essere dichiarato in un blocco (§III.0) versionato indipendentemente, e una nuova variante aggiunta in una versione successiva non deve rompere la compilazione del codice che già usa quell'enum altrove. Si rinuncia alla verifica di esaustività a compile-time in cambio di un comportamento sempre definito, anche per varianti non ancora previste quando il codice è stato scritto. Di conseguenza, il compilatore **non** emette un warning per `default` ritenuto inutile in uno `switch` apparentemente esaustivo.

## 8. Operatori

**Matematici:** `+ - * / % **` (PEMDAS). `**` è **associativo a destra**: `2 ** 3 ** 2` = `2 ** (3 ** 2)` = `512`. `%` è definito anche su float (resto in virgola mobile, stile `fmod`). Esponente negativo con `**` su tipo intero: crash a compile-time se l'esponente è un literal costante (il risultato non sarebbe rappresentabile come intero), errore di compilazione se determinabile staticamente, altrimenti crash a runtime.

**Comparazione:** `== != > < >= <=` → `bool`. Su stringhe solo `==`/`!=` byte a byte. **Logici:** `&& || !`, short-circuit. **Bitwise:** `& | ^ ~ << >>`.

**Cast:** due operatori distinti.
- `as` — crasha se il valore non entra nel tipo di destinazione (§4). Per conversioni dove la perdita di dati sarebbe un bug.
- `wrapping_as` — non crasha mai: tronca ai bit bassi del tipo di destinazione (comportamento wrap-around, stile C). Per conversioni dove il troncamento è voluto.

Divisione per zero e overflow: crash + checkpoint.

## 9. Funzioni Built-in

**Matematica:**
```text
sqrt abs round floor ceil trunc sign
sin cos tan asin acos atan atan2
log ln log2 exp
min(a,b) max(a,b)  min(array) max(array)
random() random(max) random(min,max) seed(v)
map(v, in_min, in_max, out_min, out_max)
constrain(v, min, max)
```
`random()` → `32bit.float` in `[0, 1)`. `random(max)` → stesso tipo di `max`, in `[0, max)`. `random(min, max)` → `[min, max)`. Tutti e tre esclusivi in alto, stessa convenzione di `0..N` (§12). `seed(v)` → `v` è `unsigned.32bit`, inizializza il generatore pseudocasuale usato da tutte le varianti di `random()`. `map`/`constrain` sono generici su qualunque tipo numerico (intero o float): `v`, `in_min`, `in_max`, `out_min`, `out_max` devono avere tutti lo stesso tipo tra loro nella stessa chiamata.

**Stringhe e array:**
```text
length(s)           // stringa: BYTE (UTF-8). array: elementi
char_length(s)       // stringa: caratteri (code point UTF-8)
max_length(s)        // stringa: buffer massimo dichiarato, in byte (255/65535/4G)
concat(s1, s2)
push(arr, value)  pop(arr)  insert(arr, i, value)  remove(arr, i)  clear(arr)
sum average min max
take unlink
```
`length(s)` conta byte; per contare caratteri Unicode usare `char_length(s)` (conta code point, **non** grapheme cluster: un'emoji composta da più code point conta come più di 1); per il tetto del buffer usare `max_length(s)`. Le stringhe vuote sono valide per `concat`: `concat("", s)` e `concat(s, "")` producono una copia di `s` (e `concat("", "")` una stringa vuota). `concat` il cui risultato supera `max_length` del tipo di destinazione: crash, stessa categoria di `push` su array pieno (§6). `insert(arr, i, value)` sposta gli elementi da `i` in poi di una posizione; `i` fuori range (`i > length(arr)`): crash. `pop(arr)` rimuove e ritorna l'ultimo elemento. `remove(arr, i)` rimuove l'elemento all'indice `i`, spostando i successivi (vietato su array-nodo, §II.12). `sum`/`average`/`min`/`max` funzionano identicamente su array di interi e di `float`, nessuna differenza di comportamento. `take` funziona anche su array di `struct`: lo slice risultante espone gli stessi campi dell'elemento, con le stesse regole di mutabilità per campo di §4/§5.

**I/O:**
```text
read_file(path)  write_file(path, contenuto)  include(path)
display(...)     // variadica, separatore spazio, newline finale, solo valori scalari (numerici, bool, string)
```
`display()` non accetta `struct`/`array`/`tuple` direttamente: per stamparli, passare i singoli campi/elementi come argomenti separati, es. `display(sensor.label, sensor.average);` — restano built-in i *valori* dei campi anche se il contenitore non lo è. `write_file` **sovrascrive** se il file esiste, lo **crea** se non esiste (nessuna modalità append in 0.1). `read_file`/`write_file`/`include` risolvono i path **relativi alla directory del progetto** (dove sta `main.ldc`), non alla working directory del processo.

**Introspezione:** `type_of sign_of bit_size_of is_mut identity_of`. **Hardware:** `set_pin_mode write_pin read_pin read_analog set_pwm delay tone pulse_in read_dht` + I2C/SPI/UART/servo. **Controllo:** `checkpoint reset` (istruzioni), `pause stop` (listener), `is_online(#nome) -> bool` (bridge, §II.5).

## 10. Introspezione

Risolte a compile-time, zero costo runtime.

```text
type_of(x)       // "unsigned.8bit"
sign_of(x)       // "signed" | "unsigned"
bit_size_of(x)   // intero: larghezza in bit; stringa: tag di categoria 8, 16 o 32
is_mut(x)        // bool
identity_of(x)   // struct TypeIdentity, con metadati statici del tipo e del binding
```

```text
struct TypeIdentity {
    type: let.const.16bit.string,
    sign: let.const.16bit.string,
    bits: let.const.8bit,
    mut:  let.const.bool
}
```
`identity_of(x)` restituisce una `TypeIdentity` contenente `type` (lo stesso nome di tipo restituito da `type_of`), `sign` (`"signed"`, `"unsigned"` o `"none"` per `bool` e `string`), `bits` (larghezza numerica o tag di categoria della stringa; `1` per `bool`) e `mut` (se il binding è mutabile). `TypeIdentity` è una `struct` ordinaria — nessuna magia del compilatore — segue le normali regole di mutabilità per campo (§4/§5): i suoi campi sono semplicemente tutti dichiarati `const` nella definizione.

`type_of(x)` per un tipo con `unsigned` omesso include comunque il segno per esteso: `let.mut.32bit x` → `"signed.32bit"` (mai omesso, anche se `unsigned` non è stato scritto nella dichiarazione). `sign_of(x)` su `bool`/`string`: errore di compilazione (non hanno segno); su `float`: sempre `"signed"` (non esiste `unsigned.float`). `bit_size_of` su stringa ritorna 8, 16 o 32 come **tag della categoria**, non come dimensione del buffer né come lunghezza attuale: per i byte usati, `length`/`char_length`; per il tetto massimo del buffer, `max_length` (§9). Non si può passare a nessuna di queste funzioni una variabile il cui tipo dipende da un ramo runtime.

## 11. Variabili Built-in

| Variabile | Tipo | Unità/note |
|---|---|---|
| `millis`, `micros` | `unsigned.32bit` | ms / µs dal boot |
| `time` | `unsigned.32bit` | secondi dal boot (ora corrente se RTC disponibile) |
| `date` | `16bit.string` | leggibile, `"YYYY-MM-DD"` |
| `free_ram` | `unsigned.32bit` | byte |
| `voltage` | `32bit.float` | Volt |
| `battery` | `unsigned.8bit` | percentuale 0–100 |
| `temperature`, `dht_temperature` | `32bit.float` | °C |
| `humidity`, `dht_humidity` | `32bit.float` | percentuale 0–100 |
| `light`, `sound` | `unsigned.16bit` | valore raw del sensore, non normalizzato |
| `pressure` | `32bit.float` | hPa |
| `PI`, `E`, `TAU` | `32bit.float` | precisione singola, non esiste `64bit.float` in 0.1 |

**Costanti hardware** (`HIGH`, `LOW`, `OUTPUT`, `INPUT`, `A0`, `GPIO_13`): da tabella built-in per `env.target`. Usarne una non definita: errore di compilazione. Tipi: `HIGH`/`LOW` sono `bool`; `OUTPUT`/`INPUT` sono `unsigned.8bit` (modalità pin, valori specifici del target); i nomi di pin (`A0`, `GPIO_13`) sono `unsigned.8bit` (numero di pin fisico sul target).

Un blocco (tipicamente di tipo Driver, §III.0) può aggiungere proprie costanti tramite `provides.constants` nel suo `block.ldc`: una volta importato, si usano come le costanti hardware native. In caso di conflitto di nome con una costante nativa o di un altro blocco, si disambigua con `blocco::COSTANTE` (stessa regola di §III.0).

## 12. Controllo di Flusso

**If/else** (parentesi obbligatorie, usabile come espressione):
```text
let mode = if (time > 20.00) { "night" } else { "day" };
```
Un literal stringa senza contesto di tipo esplicito (come in questo esempio) assume `16bit.string` per default.

**Loop — tre costrutti:** `repeat { } until (cond);` (do-while) · `while (cond) { }` (condizione-prima) · `for (elem in collezione) { }` (array o range).

**Range `a..b`:** sempre **esclusivo in alto** — `0..10` copre `0..9`, `10` escluso. Stessa convenzione per il secondo argomento di `take` e per `random(max)`/`random(min, max)` (§9): un'unica regola in tutto il linguaggio.

`for (elem in collezione)`: `elem` è una **copia** — modificarlo non altera la collezione. Per mutare gli elementi durante l'iterazione, iterare per indice: `for (i in 0..length(arr)) { arr[i] = ...; }`.

**Loop e watchdog:** un loop (`while`, `repeat`, **o `for`**) senza `delay()` né confine di chunk al suo interno non raggiunge mai un punto di yield (§II.0) — nessun `listener` gira finché non esce. Il watchdog automatico (§II.9) viene "nutrito" a ogni yield: un `while(true) {}` (o un `for` su una collezione enorme senza yield interno) privo di yield lo lascia scadere, causando un reset hardware, trattato come crash (§II.8: recovery dall'ultimo checkpoint). È una rete di sicurezza deliberata contro i loop bloccanti, e vale per tutti e tre i costrutti allo stesso modo.

Per comportamento continuo non bloccante: `listener` (§II.0).

## 13. Funzioni

**Dichiarazione:** ogni `fn` ha `-> tipo`, incluso `-> void`. Eccezione: `fn main`.
```text
fn compute_speed(d: 32bit, t: 32bit) -> 32bit { return d / t; }
fn log_event(msg: string) -> void { display("[log]", msg); }
```
Nessun parametro di default, nessuna funzione variadica in 0.1 (eccetto `display()`, che è built-in e non una `fn` definibile dall'utente): ogni chiamata deve passare tutti i parametri dichiarati. Stessa restrizione per `closure`.

**Funzioni come valori:**
```text
let.const.fn(32bit, 32bit) -> 32bit operation = compute_speed;
```

**Arrow function**, si assegna come qualunque valore:
```text
let double = (x: 32bit) -> 32bit => x * 2;
```

**Closure** (cattura per riferimento):
```text
let.mut.32bit factor = 3;
let multiplier = closure (x: 32bit) -> 32bit => x * factor;
```
Non può sopravvivere al chunk in cui nasce — errore di compilazione. Conseguenza diretta: una closure **non può essere restituita (`return`) da una funzione**, perché il frame della funzione è il chunk in cui nasce. Per passare comportamento tra scope, usare una `fn` semplice (puntatore a funzione, senza cattura). Questo vale anche per una `closure` che di fatto non cattura nulla: la regola guarda la sintassi (`closure`, non `fn`), non se una cattura è effettivamente usata — per un caso del genere si scrive direttamente una `fn`.

**Ricorsione:** ogni chiamata (ricorsiva o no) apre il proprio frame **nell'arena del chunk in cui la chiamata avviene**, non in un'arena a parte: una catena di chiamate ricorsive fatta dallo stesso chunk impila i frame LIFO in quell'arena. Limite `max_recursion_depth` (in `security { }`), conta frame annidati. Sforarlo: overflow.

**Return per valore.** `struct`, `array`, `tuple` restituiti da una `fn` sono **copiati** nell'arena del chunk chiamante — nessun riferimento implicito. Per strutture grandi su MCU con poca RAM, evitare il return per valore e passare un `&mut` di output come parametro invece.

## 14. Riferimenti

Quattro forme: **`&T`** (sola lettura), **`&mut T`** (scrittura, una sola attiva per variabile), **`&[T]`** (slice sola lettura, da `take`), **`&mut [T]`** (slice scrittura, da `take`).

Riferimento non sopravvive al chunk della variabile puntata — errore di compilazione.

`&mut x` (sull'intera struct) e `&mut x.campo` (su un suo campo) sono considerati lo **stesso borrow** ai fini del conflitto: non esiste split-borrow in 0.1 (nessun modo per il compilatore di dimostrare che due `&mut` su campi diversi della stessa struct non si sovrappongono mai). Due `&mut` attivi contemporaneamente su `x` e su un suo campo, in qualunque combinazione: errore di compilazione. Limite noto, valutabile in versioni future.

## 15. Gerarchia dei Chunk

Se `env.ram < 100000` (100 KB), Laplace inserisce **automaticamente** confini di chunk aggiuntivi dove utile a liberare memoria prima (oltre a quelli scritti a mano con `#nome{}`). Sopra i 100 KB questo inserimento automatico non scatta, ma il modello a chunk **resta identico e disponibile a qualsiasi dimensione**: `#nome{}` scritto a mano funziona sempre, è solo l'aggiunta automatica di Laplace a diventare superflua quando la RAM è abbondante. I due meccanismi sono ortogonali: uno decide *se Laplace segmenta anche da sola*, l'altro (`#nome{}`) è sempre nelle mani del programmatore.

Chunk annidati ad albero:
```text
fn main {
    #trip {
        let.mut.32bit distance = 0;
        #measurement {
            distance = distance + 120;
        }
        let.const.32bit final_distance = fn_main/#trip/#measurement/distance;
    }
}
```

- `#nome` — prefisso evita ambiguità. Radice: `fn_main`.
- Ogni chunk: arena propria, LIFO, zero frammentazione.
- Figlio eredita in lettura. Modifiche a variabili ereditate si perdono alla fine del chunk **a meno di un pull esplicito** fatto nell'istante di chiusura (vedi punto successivo) — senza pull, la modifica è persa.
- Path `/` per pull esplicito, solo dopo chunk terminato. Il valore viene catturato dal compilatore nell'istante in cui il chunk termina, prima che la sua arena venga liberata (LIFO): il pull non è una lettura a runtime dentro un'arena già rilasciata, ma un valore di uscita del chunk, leggibile **una sola volta** subito dopo la sua chiusura — un secondo tentativo di lettura dello stesso pull (stesso path, stessa chiusura di chunk) è un errore di compilazione, rilevato staticamente, non un problema a runtime.
- **Non esiste `free()`**: solo chunk figlio più piccolo per liberare prima.

---

# Parte II — Runtime

## 0. Listener

Loop infinito che ricontrolla la propria condizione, con priorità assoluta — unico costrutto che può modificare `rule{}`. Ogni `listener` ha nome proprio, priorità = ordine di dichiarazione.

`motor_pin` in questo esempio è una variabile top-level dichiarata in `identity{}` (es. `let.const.unsigned.8bit motor_pin = 9;`), omessa qui per brevità — un `listener` legge/scrive solo variabili top-level, mai locali di chunk (vedi "Scope dei listener" sotto).

```text
listener #battery_check (battery < rule.min_battery) {
    set_pwm(motor_pin, 0);
    display("BATTERIA CRITICA! Sistema arrestato");
    pause(#temp_check);

    if (battery >= 80) {
        rule.max_speed = 25;
        pause(self);
    }
}

listener #temp_check (temperature > rule.max_temp) {
    rule.max_speed = 10;
    display("TEMPERATURA ALTA! Velocità ridotta");
    if (temperature <= 70) {
        rule.max_speed = 25;
        pause(self);
    }
}
```

`pause(#nome)` sospende, `stop(#nome)` termina in modo permanente. Entrambe accettano anche `self`: `pause(self)` sospende il listener corrente e `stop(self)` lo termina. `self` è una keyword riservata, non una variabile — si risolve al listener che la contiene. Un `listener` può riferirsi a un altro `listener` dichiarato **più avanti** nel file (come `#battery_check` fa con `#temp_check` sopra): la risoluzione dei nomi avviene in una prima passata su tutto il file, la forward reference è permessa. La condizione tra parentesi è **una singola espressione booleana**: per una condizione composta si usano `&&`/`||` (§I.8) dentro quell'unica espressione — non esiste una sintassi per più condizioni separate sullo stesso `listener`.

**Modello:** listener **cooperativi**, non preemptive. Ricontrollati a punti di yield: al confine di ogni chunk e a ogni tick di `delay()` (§3). "Priorità assoluta" = ordine tra listener allo stesso yield. Se il programma non raggiunge mai un punto di yield (nessun chunk, nessun `delay()`), i listener semplicemente non girano mai — e lo stesso ciclo che li affama fa scadere il watchdog (§I.12), quindi la situazione si risolve comunque con un crash/recovery, non con un blocco silenzioso indefinito.

Race-condition detection = controllo statico: se due listener scrivono lo stesso campo di `rule{}`, il compilatore avvisa **a meno che** tra i due esista una relazione esplicita di `pause`/`stop` che garantisce non possano essere entrambi attivi contemporaneamente (come `#battery_check` che fa `pause(#temp_check)` sopra). Il solo ordine di dichiarazione (che decide la priorità *quando entrambi sono attivi allo stesso yield*) non basta a togliere il warning: la priorità decide chi gira per primo, non se la scrittura dell'altro sia comunque possibile in un giro successivo.

**Scope dei listener:** un `listener` legge e scrive solo variabili top-level (`fn main`/`identity{}`), `rule{}` e variabili built-in — non ha accesso alle variabili locali del chunk eventualmente in esecuzione quando scatta. Non esiste quindi il problema delle modifiche perse sulle variabili ereditate (§I.15): i listener semplicemente non vedono lo stato locale di un chunk.

**Interrupt hardware:** per eventi edge-triggered, esiste `isr`:
```text
isr #button_press (on: GPIO_13, edge: rising) {
    button_pressed = true;
}
```
`edge:` accetta `rising`, `falling`, `change` (entrambi i fronti). Un `isr` può scattare durante un chunk qualsiasi: nel corpo solo scritture a variabili già esistenti, mai nuove arene. Esecuzione atomica rispetto al resto del programma (il chunk interrotto riprende esattamente da dove si trovava): nessuna race condition a livello LDC tra `isr` e il codice principale, la mutua esclusione è garantita dal runtime generato. Autorizzazione per modulo, non globale:
```toml
[security_rule.isr]
allow_isr = false
isr_allowed_modules = ["./hardware_driver.ldc"]
```
`isr` in file non elencato: errore di compilazione. Ogni `isr` è legato a **un solo pin** (`on: GPIO_13`): per più pin servono più blocchi `isr #nome{}` distinti, uno ciascuno.

## 1. On-Event

Stesso schema di `listener`, ma scatta **una volta sola** e si disattiva da solo. Modello cooperativo, stessi punti di yield. Come per `listener`, la condizione è una singola espressione booleana — condizioni multiple si combinano con `&&`/`||`.

```text
on_event #download_complete (download_status == "completed") {
    display("Download finito!");
    display("File salvato in:", download_path);
}

on_event #wifi_ready (wifi_connected == true) {
    display("WiFi connesso, IP:", ip_address);
}
```

Niente `pause()`/`stop()` su un `on_event`: si disattiva automaticamente da solo, e chiamare `pause`/`stop` su un suo nome è un errore di compilazione (quelle due funzioni si applicano solo a `listener`). Riattivabile solo ridefinendolo — sempre a **compile-time**: due `on_event #nome{...}` con lo stesso nome nello stesso file (tipico quando il secondo arriva da un `include()`, §II.4) fanno sì che la seconda definizione sostituisca la prima nel binario finale. Non esiste un meccanismo per "riarmare" a runtime lo stesso `on_event` dopo che è scattato.

## 2. Rule{} e Sicurezza

`rule{}` vive in `identity{}` (§I.2), modificabile **solo** da `listener`, accesso `rule.campo`. Ogni campo di `rule{}` può essere di **qualunque tipo scalare built-in** (numerico di qualsiasi dimensione, `bool`, `string`) — gli esempi di questo manuale usano `unsigned.8bit` perché adatto ai valori mostrati (velocità, temperature), non per un vincolo del linguaggio. Modifiche persistono al riavvio, scritte in **EEPROM** (non Flash): EEPROM ~100.000 cicli, Flash ~10.000. Su target senza EEPROM, `rule{}` torna ai valori sorgente solo in RAM. Il warning è di **analisi statica a compile-time**: Laplace segnala ogni percorso di codice in cui un `rule.campo` viene letto prima che esista, in quel percorso, una scrittura precedente — non è un controllo che avviene a runtime. Stesso storage e fallback per `checkpoint;` (§8).

```text
if (current_speed > rule.max_speed) {
    display("ATTENZIONE: superato il limite!");
}
```

**Quando si scrive in EEPROM:** le modifiche a `rule.campo` fatte durante l'esecuzione di un `listener` restano bufferizzate in RAM e vengono scritte in EEPROM in un solo blocco quando il `listener` termina — non a ogni singola assegnazione — per ridurre il consumo di cicli di scrittura. Una lettura di `rule.campo` nella stessa invocazione, dopo averlo modificato, vede subito il valore aggiornato in RAM, anche prima del commit su EEPROM.

**Wear leveling:** `rule{}` e `checkpoint;` (§8) di default scriverebbero sempre allo stesso indirizzo EEPROM — il caso peggiore per l'usura. `security { eeprom_wear_leveling_slots = N; }` (default 8) fa ruotare le scritture su N slot, moltiplicando per N i cicli disponibili; al boot Laplace individua lo slot valido più recente tramite un contatore/CRC in testa a ciascuno slot. Il compilatore avvisa comunque se rileva `rule.campo = ...` o `checkpoint;` dentro un loop stretto senza `delay()`, dato che ogni ciclo consuma scritture.

**Blocco `security { }` in `main.ldc`:** l'utente scrive la sicurezza nel sorgente, non in TOML separato. Laplace lo estrae e lo mette in `[security_rule]` di `ldc.toml`, preservandolo ad ogni rigenerazione.

```text
security {
    max_hardware_speed = 30;
    max_hardware_temp = 85;
    can_modify_max_speed = false;
    can_disable_watchdog = false;
    log_all_rule_changes = true;
    log_path = "./logs/security.log";
    auto_lock_after_crash = 3;
    reset_speed_after_crash = true;
    bridge_keys = "injected";
    authorized_listeners = ["#battery_check", "#temp_check"];
    allow_unsafe_blocks = false;
    unsafe_allowed_modules = ["./hardware_driver.ldc"];
    allow_isr = false;
    isr_allowed_modules = ["./hardware_driver.ldc"];
    chunk_ram_ratio = 0.6;
    max_string_multiplier = 0.5;
    max_recursion_depth = 32;
    max_delay_ms = 10000;
    eeprom_wear_leveling_slots = 8;
    authorized_blocks = [
        "os@blake3:aaa111",
        "network@blake3:bbb222"
    ];
    whitelist_enabled = true;
}
```

**`[security_rule]` in `ldc.toml`** è immutabile: protetta da password OS, nessun codice/`listener` può modificarla a runtime, ogni scrittura loggata. Preservata ad ogni rigenerazione, mai riscritta. Modificabile solo cambiando il blocco `security { }` in `main.ldc` via `laplace security edit` (richiede password OS).

La protezione non ferma un attaccante con accesso root/fisico (nessuna misura software può farlo senza secure element): serve contro modifiche accidentali o non privilegiate, e a garantire il log.

**Cosa significa "immutabile a runtime" in pratica.** `[security_rule]` esiste come sezione di `ldc.toml` solo in fase di **sviluppo**: a compile-time, Laplace legge quei valori e li compila come costanti dentro il binario finale — sul dispositivo non c'è alcun file `ldc.toml` da "leggere", quindi non c'è nulla che codice o `listener` potrebbero modificare a runtime nemmeno volendo. La password OS e il log proteggono l'**unica fase in cui questi valori sono modificabili**: quando uno sviluppatore, sulla propria macchina, rigenera il binario.

**Significato dei campi meno ovvi:** `auto_lock_after_crash = N` — dopo N crash consecutivi senza un `reset;` andato a buon fine nel mezzo, il dispositivo si blocca in stato sicuro (nessun comando hardware eseguito) finché non viene riflashato; `reset_speed_after_crash = true` — dopo un crash, `rule.max_speed` torna al suo valore sorgente invece di restare al valore che aveva un `listener` impostato prima del crash; `can_disable_watchdog = false` — impedisce a qualunque codice di disattivare il watchdog automatico di §II.9 (che quindi non ha, ed è per questo che serve la voce qui, un modo per farlo se non tramite questo flag esplicito).

`log_path` è relativo alla directory del progetto, come gli altri path di §I.9. Se `log_all_rule_changes = true` ma il target non ha un filesystem scrivibile (tipico su MCU nudo senza SD/flash filesystem): errore di compilazione esplicito ("il target non supporta la destinazione di log richiesta"), non un fallimento silenzioso a runtime.

**`bridge_keys = "injected"`**: le chiavi non vivono in chiaro. Il valore vero è scritto nella memoria del dispositivo al flashing (dal toolchain, fuori da qualsiasi file). `"injected"` = stringa, non keyword.

| | `rule{}` | `security { }` |
|---|---|---|
| Modificabile da | `listener` | Nessuno |
| Dove | Sorgente `.ldc` | `security { }` in `.ldc` → `[security_rule]` in `ldc.toml` |
| Scopo | Limiti software | Limiti hardware critici |

## 3. Comandi Hardware

Sintassi a chiamata di funzione, ritorno diretto.

```text
set_pin_mode(13, OUTPUT);
write_pin(13, HIGH);
let button = read_pin(5);
let raw = read_analog(A0);
let voltage = raw * 5.0 / 1023.0;
set_pwm(motor_pin, speed);
delay(1000);
delay(rule.interval);  // richiede un campo `interval` dichiarato in rule{} (§2), non presente negli esempi di questo manuale
delay(wait_time);   // OK anche se mut: valore fisso al momento della chiamata
```

`delay()` non impone restrizioni su mut/costante. Il limite è in `security { max_delay_ms = 10000; }`. È attesa a yield, non sleep bloccante: ricontrolla i `listener` a ogni tick.

**Firme dei comandi base:** `set_pin_mode(pin: unsigned.8bit, mode: bool)`, `write_pin(pin: unsigned.8bit, value: bool)`, `read_pin(pin: unsigned.8bit) -> bool`, `read_analog(pin: unsigned.8bit) -> unsigned.16bit`, `set_pwm(pin: unsigned.8bit, value: unsigned.8bit)`, `tone(pin: unsigned.8bit, freq_hz: unsigned.16bit)`, `pulse_in(pin: unsigned.8bit, value: bool) -> unsigned.32bit` (µs).

Comandi avanzati: I2C, SPI, UART, shift register, servo, DHT (`read_dht()` popola `dht_temperature`/`dht_humidity`). Ogni comando richiede la capacità dichiarata in `capabilities` — altrimenti errore di compilazione. Le firme esatte di I2C/SPI/UART/servo restano intenzionalmente fuori da questo manuale in 0.1: sono superfici più ampie (multi-funzione, con opzioni di configurazione per bus) che meritano una sezione propria in una revisione futura, non una riga per comando qui.

## 4. File I/O e Include

**Built-in:** `read_file(path)`, `write_file(path, contenuto)`.

```text
let config = read_file("config.txt");
write_file("log.txt", "Temperatura: 25 C\n");
include("./sensori.ldc");
include("./display.ldc");
display("Sensore:", sensori.temperatura);
```

`read_file` restituisce stringa di lunghezza runtime → arena espandibile del chunk corrente (§6). Su RAM insufficiente: errore compile-time — vedi subito sotto cosa "RAM insufficiente" significa esattamente, perché la dimensione del file non è nota a compile-time.

**Come decide il compilatore:** la dimensione di un file arbitrario non è nota a compile-time, quindi il controllo non verifica "questo file specifico entra" (impossibile da sapere in anticipo), ma solo se l'arena del chunk ha **in linea di principio** margine per un'espansione runtime. Se il margine garantito è già zero (come nell'esempio arduino_uno, `env.ram = 2048`, §6), è un errore di compilazione: nessuna lettura runtime potrebbe mai riuscire. Se il margine esiste, il compilatore lascia passare, e la verifica sulla dimensione effettiva del file avviene a runtime: se non entra, è un crash da capacità superata (stessa categoria del `push` su array pieno, §I.6), non un errore catturabile con `try`/`catch`.

Namespace automatico = nome file senza estensione. Locali hanno priorità su importate in collisione.

## 5. Multi-Ambiente e Bridge

`env.target` è **statico**: descrive hardware fisico, fisso al flashing.
```text
env { target: "arduino_uno"; }
```

Per comportamento runtime si usa `identity.mode`:
```text
identity {
    let.const.string mode = if (time > 20.00) { "night" } else { "day" };
}
```

**Codice per ambiente:** `if` normale su `env.target`.
```text
if (env.target == "arduino_uno") {
    set_pin_mode(13, OUTPUT);
    display("LED Arduino acceso");
}
```
Hardware dichiarato su più ambienti senza specificazione: errore di compilazione.

**Bridge tra ambienti:** blocco con nome, comunicazione cifrata. Chiavi mai nel sorgente: referenziate da `security { bridge_keys = "injected"; }`.

| | `e2e_ecc` (default) | `e2e_rsa` |
|---|---|---|
| Algoritmo | Curve25519 | RSA-2048+ |
| Chiave | ~32 byte | ~256 byte |
| Costo MCU | Basso | Alto |
| Uso | Sempre | Solo per compatibilità |

`e2e_rsa` su target senza coprocessore: warning compile-time.

```text
bridge #telemetry {
    from: "arduino";
    to: "smartphone";
    method: "bluetooth";
    encryption: "e2e_ecc";
    bridge_keys: "injected";
    data: [temperature, battery];
    frequency: 1000;
    timeout: 5000;
    on_offline: ConnectionPolicy::Continue;
}
```

`frequency` e `timeout` sono in **ms** (coerente con la convenzione di §I.2). `bridge_keys` in `security{}` è il default globale per tutti i bridge del progetto; `bridge_keys` dentro un singolo `bridge #nome{}` è un **override locale** solo per quel bridge — utile quando bridge diversi devono usare chiavi diverse. Se il bridge dichiara lo stesso valore del default globale (come nell'esempio, entrambi `"injected"`), l'override è ridondante ma non è un errore.

Il bridge invia **media pesata temporale** (valori recenti contano di più, campionamento ogni `frequency/10` ms), accessibile con `telemetry.temperature`. I campi in `data: [...]` vengono serializzati nel loro tipo built-in nativo (stessa codifica binaria usata per `checkpoint;`, §8) — nessuna conversione implicita di tipo tra i due lati del bridge. Bridge multipli: più blocchi `bridge #nome { }`.

**Pacchetti persi:** assorbiti naturalmente dalla media pesata temporale — un campione mancante è solo un buco nella serie, nessuna gestione speciale necessaria.

**Offline prolungato** (oltre `timeout`, per tentativi ripetuti): `on_offline` decide cosa succede. `ConnectionPolicy::Continue` — il programma prosegue normalmente, `telemetry.campo` restituisce l'ultimo valore ricevuto finché non torna online. `ConnectionPolicy::Suspend` — il bridge smette di ritentare finché non viene ripreso esplicitamente. Per verificare lo stato in un `listener`/`on_event`: `is_online(#nome)` (bool).

## 6. Gestione Memoria

Limiti in `security { }` (§2):
```text
chunk_ram_ratio = 0.6;        // frazione di env.ram riservata a un chunk, 0 < x ≤ 1
max_string_multiplier = 0.5;  // frazione di CHUNK_SIZE per una stringa, 0 < x ≤ 1
```
```text
CHUNK_SIZE = env.ram * chunk_ram_ratio
MAX_STRING_LENGTH = CHUNK_SIZE * max_string_multiplier
```

`chunk_ram_ratio` e `max_string_multiplier` sono `32bit.float`. `CHUNK_SIZE` e `MAX_STRING_LENGTH`, essendo quantità di byte, sono sempre **troncati verso il basso** (mai arrotondati per eccesso, per non promettere più memoria di quanta realmente riservata) al più vicino intero.

Entrambi i fattori sono **frazioni**, non moltiplicatori liberi: `chunk_ram_ratio` e `max_string_multiplier` devono stare in `(0, 1]`. Questo garantisce **per costruzione** `MAX_STRING_LENGTH ≤ CHUNK_SIZE ≤ env.ram` — nessuna stringa può mai avere un limite dichiarato più grande dell'arena che dovrebbe contenerla, ed è verificabile a compile-time senza bisogno di un controllo separato sul valore calcolato.

`chunk_ram_ratio > 1` (o ≤ 0): errore di compilazione. `max_string_multiplier > 1` (o ≤ 0): errore di compilazione, "è una frazione dell'arena del chunk, non può superarla". `CHUNK_SIZE > env.ram`: errore di compilazione (conseguenza diretta del vincolo su `chunk_ram_ratio`, mantenuto come controllo esplicito e messaggio dedicato).

Memoria liberata **tutta insieme** alla fine del chunk. Nessuna garbage collection. **Non esiste `free()`.** Per liberare prima, annidare in chunk figlio più piccolo:
```text
#trip {
    let.mut.32bit distance = 0;
    #load_buffer {
        let.mut.array[1000].unsigned.16bit big_buffer;
    }   // big_buffer libero, distance vivo
    display(distance);
}
```

**Nessuna scelta stack/heap.** Variabile di dimensione runtime → arena espandibile, stessa sintassi `let`, nessun tag `.stack`/`.heap`.
```text
#load_config {
    let content = read_file("config.json");
    display("Config caricata, byte:", length(content));
}
```
Su RAM limitata, se l'espansione potrebbe non entrarci — questo è il caso di margine-zero descritto in §4:
```
Error: espansione arena non garantita
  main.ldc:12  let content = read_file("config.json")
  → read_file produce stringa di lunghezza sconosciuta a compile-time
  → su arduino_uno (env.ram = 2048), l'arena di #load_config non può
    garantire l'espansione necessaria
  Suggerimento: sposta in chunk figlio dedicato, riduci il file, o
    usa target con più RAM
```

## 7. Gestione Errori

Stile unico: `try`/`catch`. Errori **previsti** (I/O, file mancante) qui; crash **non gestiti** (overflow, div/0, hardware fault) con `checkpoint`/`reset`.

```text
try {
    let cfg = read_file("config.txt");
    let data = read_file("data.txt");
} catch (err) {
    display("Errore caricamento:", err);
}

ignore_errors {
    let opt1 = read_file("optional1.txt");
}
```

**`throw`** — argomento stringa (messaggio), catturato come `err`:
```text
fn parse_config(raw: string) -> Config {
    if (length(raw) == 0) {
        throw "config vuota";
    }
}
```
In 0.1 `throw` accetta solo `string`: `err` in `catch` è sempre una stringa, nessun tipo strutturato — `err` **è** il messaggio, non un oggetto con un campo `.message` o simili; `display("Errore:", err)` stampa direttamente il testo passato a `throw`. Errori tipizzati (enum/struct) sono fuori scope per 0.1 — valutabili in versioni future senza rompere il codice esistente.

`ignore_errors` esegue e ignora errori previsti, senza `catch`: la sintassi `ignore_errors { } catch (err) { }` non è valida — se serve leggere l'errore, va usato `try`/`catch`, non `ignore_errors`. Complementare a `try`/`catch`, non alternativo.

Non esiste `finally` in 0.1: nessun blocco garantito eseguito sia in caso di successo che di errore. Per logica di cleanup, va duplicata esplicitamente dopo `try`/`catch` o dentro entrambi i rami.

## 8. Checkpoint e Reset

**`checkpoint;`** salva istantanea di **tutte le variabili top-level di `fn main` e di `identity{}`** (`rule{}` esclusa, ha persistenza EEPROM propria). Scritta in **EEPROM**, non Flash. Non salva periferiche hardware, ri-inizializzate da `env`/`capabilities` al riavvio.

```text
identity {
    let.mut.32bit total_distance = 0;
}

fn main {
    #trip {
        let.mut.32bit temp = 0;
        checkpoint;
    }
}
```

- `checkpoint;`/`reset;` sono istruzioni ordinarie: per eseguirle condizionatamente si usa un `if` normale attorno (`if (cond) { checkpoint; }`) — non esiste una forma sintattica dedicata "checkpoint condizionale".
- Un solo checkpoint attivo: ogni `checkpoint;` sovrascrive il precedente.
- Costo: proporzionale alla somma delle top-level, calcolabile a compile-time.
- Su crash: top-level tornano all'ultimo checkpoint, esecuzione riparte da lì, periferiche ri-inizializzate.

**`checkpoint;` in un `listener`:** permesso, nessuna restrizione — è un'istruzione come un'altra. Poiché i listener sono cooperativi (§0), due listener non eseguono mai in contemporanea: se il corpo di un listener non contiene un punto di yield (nessun `delay()`, nessun confine di chunk), gira per intero prima che un altro possa girare, quindi il `checkpoint;` al suo interno vede uno stato coerente, comprese le scritture precedenti della stessa invocazione. **Attenzione:** se il corpo contiene un `delay()`, un altro listener può eseguire nel frattempo e il `checkpoint;` successivo includerà anche quelle modifiche — comportamento voluto, coerente con l'ordine di esecuzione cooperativo, non una race condition, ma facile da leggere erroneamente come tale: un `checkpoint;` dopo un `delay()` in un `listener` non è mai una fotografia isolata del solo stato di quel listener.

**`reset;`** segue esattamente lo stesso percorso di un crash — stack dei chunk interrotto, periferiche/listener/connessioni ripristinati da zero, esecuzione riparte da `fn main` — con un'unica differenza: i top-level non tornano all'ultimo checkpoint ma allo **stato di dichiarazione**. Quello stato viene anche scritto come nuovo checkpoint in EEPROM, così un crash successivo non annulla il reset.

| | Salvato | Dopo crash | Dopo `reset;` |
|---|---|---|---|
| Top-level `fn main` e `identity{}` | Sì | Tornano all'ultimo checkpoint | Tornano allo stato di dichiarazione (diventa il nuovo checkpoint) |
| Variabili dentro un chunk | No | Azzerate | Azzerate |
| `rule{}` | No, separatamente | Persiste in EEPROM | Persiste in EEPROM |
| Funzioni (`fn`) | No | Codice, sempre esistente (indipendente da ogni chunk) | Codice, sempre esistente |
| Closure | No | Codice esistente, ma il *binding* resta comunque legato al chunk in cui è nato (§I.13) — dopo che quel chunk termina (crash o reset lo terminano entrambi) il binding non è più raggiungibile, a prescindere dal fatto che il codice esista | Stesso comportamento del crash |
| `listener`/`on_event`/`isr` | No | **Si riattivano tutti da zero**, incluso lo stato di `pause`/`stop`: un `listener` che era in `pause` prima del crash riparte **attivo**, come alla primissima esecuzione del programma | Stesso comportamento del crash |
| Periferiche hardware | No | Ri-inizializzate da `env`/`capabilities` | Ri-inizializzate da `env`/`capabilities` |
| Connessioni (UART/bridge/I2C/SPI) | No | Richiuse da zero | Richiuse da zero |
| Stack chunk annidati | No | Crash termina chunk + figli, si riparte da `fn main` | Reset termina chunk + figli, si riparte da `fn main` |

Principio: solo il puro dato persistente si salva. Codice, connessioni e stato hardware si ri-creano da `env`/`capabilities`/`identity`.

## 9. Ottimizzazioni

**Compile-time:** constant folding, dead code elimination, common subexpression elimination, inlining funzioni piccole, string interning.

**Runtime:** copy-on-write stringhe, small string optimization, arena allocator per chunk, variable reuse, lazy evaluation, watchdog automatico, auto-retry hardware transitorio.

Questa sezione elenca **cosa** Laplace ottimizza, non **come**: `lazy evaluation`, `auto-retry hardware transitorio` e le altre voci runtime sono dettagli implementativi trasparenti al programmatore (nessuna sintassi LDC li espone o li controlla), non parte della semantica osservabile del linguaggio. Restano fuori scope per questo manuale — cambiano liberamente da una versione di Laplace all'altra senza rompere codice `.ldc` esistente.

## 10. Tooling e Debug

```text
laplace build main.ldc --profile
laplace build main.ldc --memory-profile
laplace build main.ldc --static-analysis
laplace security edit
laplace security log
```

Errori riportano file, riga, colonna, messaggio; anche JSON.

## 11. Riferimenti e Puntatori Grezzi

Due viste sullo stesso spazio: `&T`/`&mut T` (sorvegliata), `*T`/`*mut T` (nuda). Fuori da `unsafe` solo riferimenti; dentro `unsafe`, convivono. `unsafe{}` annidato dentro un altro `unsafe{}` è sintatticamente permesso ma ridondante: non aggiunge né toglie permessi rispetto al blocco esterno che già li contiene.

**Puntatori grezzi** per indirizzi fissi:
```text
unsafe {
    let.mut.32bit.ptr portb = pointer(0x24);
    *portb = 255;
}
```
Ogni `pointer()` è `volatile`: nessun riordino/eliminazione. `pointer()` non porta un tipo proprio: il tipo del puntatore risultante è dedotto dal contesto della dichiarazione che lo riceve (`let.mut.32bit.ptr portb = pointer(0x24);` → `32bit.ptr`); non esiste una sintassi `pointer(0x24) as *32bit` separata, il tag di tipo sta nella `let`, non nella chiamata.

**Conversione `&`↔`*`** solo in `unsafe`:
```text
unsafe {
    let portb: &mut 32bit = &mut *pointer(0x24);
    *portb = 255;
}
```
Il borrow checker riprende in carico `portb` immediatamente: no sopravvivenza al chunk, no `&mut` concorrente sullo stesso indirizzo.

**`pointer()` è costruttore riconosciuto**, tracciato per **valore di indirizzo**, non per chiamata. `0x24` e `36` sono lo stesso indirizzo. Due `&mut` sullo stesso indirizzo (anche da `pointer()` distinte) → errore. Tracciatura per **tutto il modulo** autorizzato, non solo per blocco.

**Indirizzo calcolato a runtime** (es. `pointer(base + i * 4)` con `base`/`i` non costanti a compile-time): il compilatore non può verificare staticamente l'unicità dell'indirizzo. In questo caso emette un warning ("indirizzo non determinabile a compile-time, tracciamento anti-alias non garantito") e non impedisce due `&mut` potenzialmente sovrapposti — la responsabilità torna al programmatore, come per il resto di `unsafe`. Non esiste in 0.1 un modo per sopprimere questo o altri warning: coerente con il principio "niente silenzioso" (§II.14.0), ogni warning va risolto nel codice, non zittito da riga di comando.

**Permessi per modulo.** Tre flag **indipendenti**, nessuno implica gli altri — abilitarne uno non abilita gli altri due:
```text
security {
    allow_unsafe_blocks = false;   // può esistere un blocco unsafe{} nel modulo
    allow_raw_pointers = false;    // dentro unsafe{}, può usare pointer()/*T
    allow_inline_asm = false;      // dentro unsafe{}, può usare asm{}
    unsafe_allowed_modules = ["./hardware_driver.ldc"];
}
```
`unsafe` in file non elencato: errore di compilazione anche se `allow_unsafe_blocks` è `true`. Un modulo elencato con `allow_unsafe_blocks = true` ma senza `allow_raw_pointers = true` può aprire un blocco `unsafe{}` (per organizzare il codice, o in previsione di `asm{}`) ma non può usare `pointer()`/`*T` al suo interno — e viceversa per `allow_inline_asm`.

**Assembly inline.** Blocco `asm { }` dentro `unsafe`, richiede `allow_inline_asm = true` e modulo elencato in `unsafe_allowed_modules` (stessa autorizzazione di `allow_raw_pointers`). Stringa grezza passata al backend per il target corrente: nessun supporto del compilatore per allocazione registri o vincoli di I/O — resa e side-effect sono interamente responsabilità del programmatore. Sempre trattato come `volatile` (nessun riordino).
```text
unsafe {
    asm("nop");
}
```

**Niente indirezione multipla** (`**`). Tabella di indirizzi con array di puntatori singoli:
```text
unsafe {
    let.const.array[4].32bit.ptr registers = [pointer(0x24), pointer(0x25), pointer(0x26), pointer(0x27)];
    for (i in 0..4) { *registers[i] = 0; }
}
```

## 12. Linked List, Grafi e Alberi

Linked list = array di nodi con indici. Riusa array dinamico, struct, enum con dati. Niente `malloc`/`free`, niente puntatori grezzi, niente indirezione multipla.

```text
enum Link { Node(32bit), End }

struct Nodo {
    valore: let.mut.32bit,
    prossimo: let.mut.Link
}
```

**Creazione e lettura:**
```text
fn main {
    #lista {
        let.mut.array[? max 100].Nodo lista = [];
        push(lista, { valore: 10, prossimo: Link::End });
        push(lista, { valore: 20, prossimo: Link::End });
        push(lista, { valore: 30, prossimo: Link::End });
        lista[0].prossimo = Link::Node(1);
        lista[1].prossimo = Link::Node(2);

        let.mut.Link cursore = Link::Node(0);
        while (cursore != Link::End) {
            switch (cursore) {
                case Link::Node(i):
                    display(lista[i].valore);
                    cursore = lista[i].prossimo;
                    break;
                case Link::End: break;
                default: break;
            }
        }
    }
}
```

**Inserimento** (due collegamenti, nessuno spostamento):
```text
push(lista, { valore: 20, prossimo: Link::Node(1) });
lista[0].prossimo = Link::Node(2);
```

**Rimozione:** `lista[0].prossimo = lista[1].prossimo;`

**`remove()` vietato su array-nodo.** Array-nodo = struct con almeno un campo di tipo `Link`, riconosciuto a compile-time **ricorsivamente** a qualunque profondità di annidamento (un `Link` dentro un array interno allo struct, o dentro un'altra struct annidata, marca comunque il contenitore esterno come array-nodo). `remove()` sposta elementi → `Link::Node(i)` punterebbero all'indice sbagliato.
```
Error: remove() non ammesso su array-nodo
  → lista è di tipo Nodo (struct con campo Link)
  Suggerimento: usa `lista[i].prossimo = lista[j].prossimo;` seguito da
    `unlink(lista, i);`
```

**Free-list:** `unlink()` registra slot liberi, `push()` li riusa prima di crescere.
```text
lista[0].prossimo = lista[1].prossimo;
unlink(lista, 1);
push(lista, { valore: 99, prossimo: Link::End });   // riusa indice 1
```
`unlink()` su array non-nodo: errore di compilazione. `unlink()` su uno slot già libero: no-op (nessun effetto, nessun errore — reso idempotente). `unlink()` con indice fuori dai limiti dell'array: crash, stessa categoria degli altri accessi fuori range (§I.5).

**Grafi e alberi — stesso schema:**
```text
struct NodoGrafo {
    valore: let.mut.32bit,
    vicini: let.mut.array[? max 8].32bit
}

struct NodoAlbero {
    valore: let.mut.32bit,
    sinistro: let.mut.Link,
    destro: let.mut.Link
}
```

## 13. Esempio Completo

```text
env {
    target: "arduino_uno";
    ram: 2048;
    hardware: [GPIO_13, GPIO_17, ADC_A0, ADC_A1];
    sensors: [DHT22_temp, DHT22_humidity];
    watchdog_timeout: 5000;
}

dependencies {
    sensors_lib = "1.2.0";
}

capabilities {
    import: "sensors_lib";
    uses: pwm;
    uses: sensors_analog;
    uses: sensors_digital;
}

security {
    max_hardware_speed = 30;
    max_hardware_temp = 85;
    chunk_ram_ratio = 0.6;
    max_recursion_depth = 32;
}

struct SensorReading {
    values: let.mut.array[5].unsigned.16bit,
    label: let.const.16bit.string,
    average: let.mut.32bit.float
}

identity {
    name: "scooter_controller";

    let.mut.32bit total_distance = 0;
    let.const.unsigned.8bit motor_pin = 9;
    let.const.array[3].SensorReading sensor_log = SensorReading::default();

    // fn dichiarata qui dentro identity{} o fuori, a livello di file, non
    // cambia nulla a livello semantico: solo organizzazione del sorgente.
    fn compute_speed(distance: 32bit, elapsed: 32bit) -> 32bit {
        return distance / elapsed;
    }

    fn average_reading(reading: &SensorReading) -> 32bit.float {
        return average(reading.values);
    }

    rule {
        let.mut.unsigned.8bit max_speed = 25;
        let.mut.unsigned.8bit max_temp = 80;
        let.mut.unsigned.8bit min_battery = 15;
    }
}

listener #battery_check (battery < rule.min_battery) {
    set_pwm(motor_pin, 0);
    display("BATTERIA CRITICA! Motore fermato");
    pause(#temp_check);

    if (battery >= 80) {
        rule.max_speed = 25;
        pause(self);
    }
}

listener #temp_check (temperature > rule.max_temp) {
    rule.max_speed = 10;
    display("TEMPERATURA ALTA! Velocità ridotta a", rule.max_speed);

    if (temperature <= 70) {
        rule.max_speed = 25;
        pause(self);
    }
}

fn main {
    // mai letta più avanti in questo esempio: warning "variabile non usata",
    // non un errore — il programma compila comunque (§I.2)
    let.const.tuple(32bit.float, 32bit.float, 16bit.string) start_point = (45.4642, 9.1900, "deposito");

    #trip {
        let.mut.32bit distance = 0;
        let.mut.32bit elapsed = 0;

        #measurement {
            // distance/elapsed sono ereditate da #trip: questa modifica,
            // senza il pull esplicito qui sotto, andrebbe persa alla
            // chiusura di #measurement (§I.15) — dentro #trip, `distance`
            // da sola tornerebbe a 0
            distance = distance + 120;
            elapsed = elapsed + 60;
            let speed = compute_speed(distance, elapsed);
            display("Velocità istantanea:", speed);
        }

        // il pull cattura il valore di distance visto DENTRO #measurement
        // nell'istante della sua chiusura, non il valore (0) a cui
        // `distance` di #trip è tornata
        let.const.32bit final_distance = fn_main/#trip/#measurement/distance;
        total_distance = total_distance + final_distance;

        display("Distanza totale percorsa:", total_distance);
        checkpoint;
    }
}
```

## 14. Accessibilità

L'accessibilità non è una feature opzionale. È un **requisito architetturale**: un linguaggio che si vuole usare per decenni, da persone diverse, in contesti diversi, deve essere accessibile a chiunque. Non "anche ai disabili". **A chiunque**. Perché l'accessibilità non è una nicchia: è la condizione di base perché un sistema sia usabile da tutti, sempre.

### 14.0 Filosofia

L'accessibilità in LDC segue tre principi:

1. **Separazione dei livelli.** Ogni livello della pipeline fa la sua parte. OS gestisce input fisico, editor gestisce rendering e filtro tastiera, Laplace gestisce messaggi e struttura, runtime gestisce errori. Nessuno fa il lavoro dell'altro.
2. **Niente silenzioso.** Ogni comportamento che tocca l'utente — un blocco, un errore, un suggerimento — deve essere **visibile** e **spiegabile**. L'utente deve sempre sapere cosa sta succedendo.
3. **Sempre reversibile.** Ogni automatismo deve poter essere **disattivato** o **forzato**. L'accessibilità non toglie mai controllo all'utente. Lo aggiunge.

Un quarto principio, implicito nella scelta di dove vivono queste preferenze (§14.2): **l'accessibilità appartiene alla persona, non al progetto.** Chi ha bisogno di uno screen reader ne ha bisogno su ogni progetto che compila, non solo su questo — quindi le preferenze non si dichiarano nel sorgente versionato, si dichiarano una volta per tutte accanto a Laplace.

### 14.1 Dove vive l'accessibilità

| Livello | Cosa gestisce |
|---|---|
| **OS** | Input fisico, sintesi vocale, lettore schermo |
| **Editor** | Rendering (font, colore, contrasto, dimensione), filtro tastiera, integrazione screen reader |
| **Laplace (compilatore)** | Struttura dei messaggi, formato output, suggerimenti, verbosità, lingua |
| **Runtime** | Errori a runtime accessibili, notifiche visive |

LDC **non** cerca di fare tutto nel compilatore. Un compilatore che tenta di gestire il rendering è un compilatore che fa male il compilatore. Ogni livello fa la sua parte, bene.

### 14.2 Il profilo di accessibilità (`~/.laplace/accessibility.toml`)

Le preferenze di accessibilità **non vivono nel progetto**. Non fanno parte di `main.ldc`, non sono un blocco della grammatica `.ldc`, e non vengono estratte in `ldc.toml`: non sono né versionate né condivise col resto del team. Vivono in un file locale legato all'installazione di Laplace, nella stessa posizione già usata per altra configurazione utente (§III.1, `~/.laplace/trusted_hashes`):

```
~/.laplace/accessibility.toml
```

```toml
language = "it"
verbosity = "normal"
explain = true
color_blind_mode = "deuteranopia"
screen_reader = true
keyboard_filter = "auto"
tremor_detection = true
spam_detection = true
sticky_keys = false
one_handed_mode = false
```

Tutti i campi sono **opzionali**. Default sicuri se assenti. Se il file non esiste, Laplace usa i default e **non avvisa** (l'accessibilità non è una scelta di sicurezza — a differenza di `security{}`, §2, non c'è un warning "nessuna configurazione trovata"). A differenza di `ldc.toml`, questo file **non è generato automaticamente da Laplace ad ogni build**: lo scrive e mantiene l'utente, una volta sola, per tutti i progetti che compilerà.

| Campo | Valori | Default | Effetto |
|---|---|---|---|
| `language` | codice ISO 639-1 (`it`, `en`, ...) | `en` | Lingua dei messaggi |
| `verbosity` | `quiet`, `normal`, `verbose`, `debug` | `normal` | Livello di dettaglio output |
| `explain` | `true`, `false` | `false` | Spiega gli errori in linguaggio naturale |
| `color_blind_mode` | `none`, `deuteranopia`, `protanopia`, `tritanopia` | `none` | Palette colori accessibile |
| `screen_reader` | `true`, `false` | `false` | Output compatibile con lettori schermo |
| `keyboard_filter` | `auto`, `strict`, `off` | `auto` | Filtro input tastiera (editor) |
| `tremor_detection` | `true`, `false` | `true` | Rilevamento tremore (Parkinson) |
| `spam_detection` | `true`, `false` | `true` | Rilevamento spam tasti (Tourette) |
| `sticky_keys` | `true`, `false` | `false` | Modificatori persistenti |
| `one_handed_mode` | `true`, `false` | `false` | Scorciatoie per una mano sola |

**Gestione da riga di comando**, senza password — a differenza di `laplace security edit` (§2), non è una decisione di sicurezza critica:
```bash
laplace accessibility show
laplace accessibility set language=it
laplace accessibility set screen_reader=true
laplace accessibility edit     # apre $EDITOR sul file
```

**Editor e Laplace leggono lo stesso profilo.** Campi come `keyboard_filter`, `tremor_detection`, `sticky_keys`, `one_handed_mode` riguardano l'editor (§14.1), non il compilatore: Laplace non li applica, li espone soltanto. Il plugin editor legge `~/.laplace/accessibility.toml` come profilo condiviso — non è Laplace a decidere per l'editor, è un profilo unico dello sviluppatore che entrambi gli strumenti consultano, allo stesso modo in cui più applicazioni condividono le impostazioni di accessibilità del sistema operativo (riga "OS" di §14.1).

**Precedenza.** Un flag da riga di comando (§14.5) sovrascrive, per quella singola build, il valore corrispondente in `accessibility.toml`. Il file resta la persistente di default tra una build e l'altra; il flag è l'eccezione puntuale per quella sola invocazione.

### 14.3 Output di Laplace: formati

Laplace produce output in **due formati**:

**Testo semplice** (default, umano):
```
Error: variabile non assegnata
  main.ldc:12:5  display(result)
  → result è dichiarata alla riga 10 ma non assegnata su ogni percorso
  → il ramo else (riga 11) non assegna result
  Suggerimento: aggiungi `result = 0;` nel ramo else
  Vedi: sezione 1 del manuale LDC
```

**JSON** (per tool, editor, screen reader):
```json
{
  "level": "error",
  "code": "LDC0001",
  "message": "variabile non assegnata",
  "file": "main.ldc",
  "line": 12,
  "column": 5,
  "context": "display(result)",
  "details": [
    "result è dichiarata alla riga 10 ma non assegnata su ogni percorso",
    "il ramo else (riga 11) non assegna result"
  ],
  "suggestion": "aggiungi `result = 0;` nel ramo else",
  "reference": "manuale LDC, sezione 1"
}
```

Il flag `--format=json` attiva il JSON. Il flag `--format=text` (default) attiva il testo. **Nessun formato è privilegiato**: sono due viste sulla stessa informazione.

### 14.4 Messaggi accessibili

**Regole:**

1. **Niente gergo inutile.** "Unexpected token" è vietato. "Manca una parentesi chiusa dopo `if (x > 0`" è richiesto.
2. **Niente colori come unico segnale.** Rosso = errore, giallo = warning è **accompagnato** da `Error:` / `Warning:` testuali. Sempre.
3. **Niente tabelle ASCII.** I lettori schermo non le leggono. Ogni informazione tabellare è **anche** in lista o in JSON.
4. **Righe corte.** Massimo 80 caratteri. Andare a capo è meglio che troncare.
5. **Frasi attive.** "Il compilatore ha trovato un errore" → "Errore:". Soggetto chiaro, verbo chiaro.
6. **Ripetizione coerente.** Se un errore si presenta due volte, il messaggio è **identico**. Niente variazioni creative.
7. **Nessuna abbreviazione ambigua.** `fn` è nel linguaggio, ok. `var` per variabile no. Ogni abbreviazione è **definita** la prima volta.

**Esempio buono:**
```
Error: tipo non compatibile
  main.ldc:8:12  let.mut.8bit x = 300;
  → 300 eccede il range di 8bit (0–255)
  Suggerimento: usa 16bit, oppure riduci il valore a 255 o meno
```

**Esempio cattivo (vietato):**
```
[E] L8:C12 type_mismatch: 0x12C >> 0xFF
```

### 14.5 Flag e modalità

Ogni flag qui sotto sovrascrive, solo per l'invocazione corrente, il campo corrispondente in `accessibility.toml` (§14.2).

```
laplace build main.ldc --accessible
```
Attiva: `language` dal sistema, `verbosity=normal`, `explain=true`, `screen_reader=true`, `color_blind_mode=none`.

```
laplace build main.ldc --explain
```
Ogni errore è seguito da una spiegazione in linguaggio naturale: cosa significa, perché è un errore, come si risolve.

```
laplace build main.ldc --lang=it
```
Messaggi in italiano. Supportate: `en`, `it`, `es`, `fr`, `de`. Altre lingue via file di traduzione.

```
laplace build main.ldc --format=json
```
Output JSON strutturato. Compatibile con editor, screen reader, tool esterni.

```
laplace build main.ldc --quiet
```
Solo errori. Niente warning, niente info.

```
laplace build main.ldc --verbose
```
Tutto: errori, warning, info, statistiche di compilazione.

```
laplace build main.ldc --debug
```
Tutto + tracce interne del compilatore. Per sviluppatori di Laplace stesso.

```
laplace build main.ldc --no-color
```
Disattiva colori anche se il terminale li supporta. Utile per screen reader e pipe.

### 14.6 Cosa NON fa Laplace

Laplace **non**:
- Rende il testo (font, colore, dimensione). → Editor.
- Filtra l'input da tastiera. → Editor.
- Sintetizza la voce. → OS.
- Legge lo schermo. → OS.
- Decide le scorciatoie. → Editor.

E **va bene**. Ogni livello fa la sua parte. Chi cerca di fare tutto, fa tutto male.

### 14.7 Editor: filtro tastiera

Il filtro tastiera vive nell'editor (plugin VSCode, o editor LDC dedicato), configurato dal profilo condiviso `~/.laplace/accessibility.toml` (§14.2). Regole:

**Regola 1 — Modificatori separati.**
Conta i modificatori (`Ctrl`, `Alt`, `Shift`, `Cmd`) separatamente dai tasti normali. Permetti sempre N modificatori + 1 tasto normale. `Ctrl+Shift+P` funziona. `Ctrl+Shift+Alt+F` funziona.

**Regola 2 — Finestra temporale.**
Se 3+ tasti **non-modificatore** arrivano in meno di 50 ms e non formano una scorciatoia nota, **blocca e mostra avviso**. Soglia configurabile in `accessibility.toml`.

**Regola 3 — Rilevamento tremore (Parkinson).**
Se lo stesso tasto viene premuto e rilasciato con intervallo regolare (5 volte in 1 secondo, intervallo ~200 ms), **sospetta tremore** e chiedi conferma: *"Stai avendo un episodio? Vuoi che rallenti l'input?"*

**Regola 4 — Rilevamento spam (Tourette).**
Se lo stesso tasto viene premuto più di X volte in Y ms senza modificatori, **blocca temporaneamente** (500 ms) e mostra feedback visivo.

**Regola 5 — Mai silenzioso.**
Ogni blocco è **visibile**: popup, icona nella status bar, suono opzionale. L'utente deve sapere che il filtro è intervenuto. Altrimenti sembra che la tastiera sia rotta.

**Regola 6 — Sempre reversibile.**
Se il filtro blocca qualcosa di voluto, l'utente può **forzarlo** con una scorciatoia (`Ctrl+Shift+FiltroOff`). L'accessibilità non toglie mai controllo.

**Regola 7 — Mai sull'input di sistema.**
Il filtro agisce **solo** sull'editor LDC. Mai su tastiera di sistema, mai su altre applicazioni. Un filtro globale è invasivo e pericoloso.

### 14.8 Editor: rendering

Il rendering è dell'editor, ma Laplace fornisce **metadati** per aiutarlo:

- **Token type** in JSON: `keyword`, `identifier`, `literal`, `comment`, `error`. L'editor decide i colori in base al tipo, non in base alla posizione.
- **Range di errore**: `{line, column, length}`. L'editor evidenzia esattamente il range, non l'intera riga.
- **Severità**: `error`, `warning`, `info`, `hint`. L'editor può filtrare per severità.
- **Palette suggerite**: Laplace propone palette color-blind-friendly, l'editor le applica.

L'editor, dal canto suo:
- **Font**: monospace, dimensione configurabile, spaziatura riga configurabile.
- **Contrasto**: minimo WCAG AA (4.5:1 per testo normale, 3:1 per testo grande).
- **Colori**: mai rosso/verde come unico segnale. Simboli testuali (`E:`, `W:`, `i:`) sempre presenti.
- **Screen reader**: VSCode ha API per questo. Il plugin LDC le usa per annunciare errori, suggerimenti, stato della compilazione.
- **Zoom**: 50%–500%, senza perdita di funzionalità.

### 14.9 Test e verifica

L'accessibilità **non è una promessa**: è una cosa che si testa.

**Test automatici:**
- Ogni messaggio di errore è analizzato per: lunghezza ≤ 80 caratteri, presenza di `Error:`/`Warning:` testuale, assenza di colori come unico segnale, presenza di suggerimento.
- Output JSON validato contro schema.
- Verifica che `--no-color` non produca sequenze ANSI.

**Test manuali:**
- Screen reader reali (NVDA, JAWS, Orca, VoiceOver) su macOS, Linux, Windows.
- Persone con disabilità reali: ciechi, ipovedenti, dislessici, con Parkinson, con Tourette, con disabilità motorie.
- Test in condizioni di stress: input veloce, input irregolare, input da tastiere diverse.

**Metriche:**
- Percentuale di messaggi conformi alle regole (target: 100%).
- Tempo medio di comprensione di un errore (test con utenti).
- Numero di richieste di aiuto su forum dopo un errore (target: zero per errori banali).

### 14.10 Checklist per nuovi messaggi

Ogni nuovo messaggio di errore deve passare questa checklist prima di essere aggiunto:

- [ ] Lunghezza ≤ 80 caratteri per riga.
- [ ] Inizia con `Error:` o `Warning:` o `Info:` (testuale, non solo colore).
- [ ] Indica file, riga, colonna.
- [ ] Spiega **cosa** è sbagliato.
- [ ] Spiega **perché** è sbagliato.
- [ ] Suggerisce **come** risolverlo.
- [ ] Non usa gergo non definito.
- [ ] Funziona in `--quiet`, `--normal`, `--verbose`.
- [ ] Funziona in `--format=text` e `--format=json`.
- [ ] Funziona con `--no-color`.
- [ ] Ha una traduzione per ogni lingua supportata.
- [ ] Ha una spiegazione `--explain` in linguaggio naturale.
- [ ] È testato con almeno uno screen reader.

Se un messaggio non passa la checklist, **non entra nel compilatore**.



---

# Parte III — Ecosistema

## 0. Blocchi

Un blocco è un'unità di capacità scaricabile, versionata, verificabile, che estende LDC. Non è una libreria: una libreria fornisce funzioni pure, un blocco fornisce **capacità** con permessi runtime, requisiti hardware, contratto di sicurezza.

**Modello a 2 file totali anche per i blocchi:**
- **`block.ldc`** — scritto dall'autore. Contiene TUTTO: metadata, requisiti, forniti, permessi, sorgenti.
- **`blockgen.toml`** — autogenerato da Laplace a `laplace block publish`. Contiene hash, tabelle finali, binari, firme. Mai scritto a mano.

**📌 La fusione in un file unico per l'ecosistema è esclusa.** Il blocco è indipendente dal progetto: ha versione, ABI, hash content-addressed. L'hash dipende dal contenuto del suo `block.ldc`, non dal progetto che lo importa. Fonderlo romperebbe content-addressed, pubblicabilità, ricerca nel registry.

**`block.ldc`** — scritto a mano:
```text
block {
    name: "os";
    version: "1.0.0";
    type: "system";
    experimental: true;
    author: "ldc-core";
    license: "MIT";
    description: "Operating system capabilities";
}

requires {
    blocks: ["crypto>=1.0.0"];
    ram: 4096;
    targets: ["linux", "bsd", "macos"];
}

provides {
    types: [
        { name: "pid",       size: "32bit" },
        { name: "exit_code", size: "8bit"  },
        { name: "result",    size: "8bit"  },
        { name: "signal",    size: "8bit"  }
    ];
    functions: [
        { name: "spawn_process", params: ["string"],                       returns: "pid" },
        { name: "wait_process",  params: ["pid"],                          returns: "exit_code" },
        { name: "mount",         params: ["string", "string"],             returns: "result" },
        { name: "umount",        params: ["string"],                       returns: "result" },
        { name: "fork",          params: [],                               returns: "pid" },
        { name: "exec",          params: ["string", "array[? max 32].string"], returns: "result" },  // sintassi per un parametro array: il tipo array[..] scritto come stringa, identica a §I.6
        { name: "signal",        params: ["pid", "signal"],                returns: "result" },
        { name: "exit",          params: ["32bit"],                        returns: "void" }
    ];
    constants: [
        { name: "AF_INET",     type: "32bit" },
        { name: "SOCK_STREAM", type: "32bit" },
        { name: "SOCK_DGRAM",  type: "32bit" }
    ];
}

permissions {
    seccomp: ["clone", "execve", "mount", "umount2", "kill"];
    capabilities: ["CAP_SYS_ADMIN", "CAP_SYS_PTRACE"];
}

build {
    sources: ["src/*.ldc"];
    abi: "1.0";
}
```

Solo `block`, `requires`, `provides`, `permissions`, `build`. Versioni in `requires.blocks` come range (es. `"crypto>=1.0.0"`, stessa sintassi di `dependencies` in `main.ldc`, §I.2); risoluzione a hash preciso a publish.

**Campi metadata di `block{}`:** `type` — uno dei sei valori della tassonomia sotto (`system`, `library`, `framework`, `wasm`, `driver`, `plugin`); `experimental = true` — il blocco compila comunque, ma ogni progetto che lo importa riceve un warning permanente a ogni build, non solo alla prima (nessuna soppressione, coerente con §II.14.0); `license` — stringa libera, si raccomanda un identificativo SPDX (`"MIT"`, `"Apache-2.0"`, ecc.) ma non è imposto dal compilatore; `author`, `description` — stringhe libere, solo informative, mostrate da `laplace info` (§2). Come per §I.2, i campi di `requires`/`provides`/`permissions`/`build` (e quelli di `blockgen.toml`/`ldc.toml`) sono documentati qui tramite esempi rappresentativi; lo schema esaustivo campo-per-campo vivrà nella grammatica formale.

**`blockgen.toml`** — autogenerato:
```toml
[meta]
generated_by = "laplace 0.1.0"
generated_at = "2026-09-11T15:00:00Z"
source = "block.ldc"
source_hash = "blake3:abc123..."

[block]
name = "os"
version = "1.0.0"
type = "system"
experimental = true

[resolved_hash]
blake3 = "blake3:abc123def456..."

[requires]
blocks = [{ name = "crypto", version = "1.0.0", hash = "blake3:def456..." }]
ram = 4096
targets = ["linux", "bsd", "macos"]

[artifacts]
binaries = [
    { target = "linux-x86_64", path = "os-1.0.0-linux-x86_64.so", hash = "blake3:aaa111..." },
    { target = "linux-arm64",  path = "os-1.0.0-linux-arm64.so",  hash = "blake3:bbb222..." }
]
sources = ["src/*.ldc"]

[signature]
algorithm = "ed25519"
public_key = "ed25519:..."
signature = "..."
```

| | `block.ldc` | `blockgen.toml` |
|---|---|---|
| Scritto da | Developer | Laplace |
| Modificabile | Sì | No |
| Versionato | Sì | No (o sì) |
| Contiene | Metadata, requisiti, forniti, permessi, build | Tutto + hash + dipendenze risolte + binari + firma |

**Nota scope.** LDC resta embedded. Blocchi Sistema/Framework che richiedono OS completo sono **sperimentali**: etichettati, compiler segnala sempre. Un blocco è sperimentale se `experimental = true` in `block.ldc`.

**Tassonomia:**

| Tipo | Privilegiato | Runtime | Compile-time | Firmato | Esempio |
|---|---|---|---|---|---|
| **Sistema** | ✅ | ✅ | ✅ | ✅ | `os`, `network`, `crypto` |
| **Libreria** | ❌ | ❌ | ✅ | ❌ | `math`, `json`, `collections` |
| **Framework** | ✅ | ✅ | ✅ | ✅ | `web`, `gui`, `db` |
| **WASM** | ❌ | ✅ | ✅ | ❌ | `wasm_codec`, `wasm_crypto` |
| **Driver** | ✅ | ✅ | ✅ | ✅ | `gpio_avr`, `i2c_stm32` |
| **Plugin** | ❌ | ❌ | ✅ | ✅ | `plugin_lint`, `plugin_opt` |

Per 0.1: supportati **sistema** e **libreria**. Tassonomia completa già fissata.

**Tipi opachi.** Blocco può restituire tipi non noti a LDC (`pid`, `exit_code`). Compilatore conosce nome e size, non layout. Niente decomposizione, niente `type_of()`. Disambiguazione: `blocco::tipo` o `blocco@blake3:hash::tipo`. `void` non è opaco: è built-in, sempre `-> void` esplicito.

**Costanti di blocco.** Blocco può dichiarare costanti built-in, disponibili solo ai file che lo importano. Usare costante senza import: errore di compilazione.

**Import:**
```text
capabilities {
    import: "os";
    import: "network";
    uses: pwm;
}
spawn_process("/usr/bin/myapp");   // OK — path assoluti hanno senso qui perché
                                    // il blocco "os" è sperimentale (richiede
                                    // OS completo, non un MCU nudo, §III.0)
socket(AF_INET, SOCK_STREAM, 0);   // OK
```
```text
capabilities { import: "crypto"; }
spawn_process("/usr/bin/myapp");   // ERRORE: richiede os
```

Niente è permesso di default. Ogni operazione richiede capability. Ogni capability richiede blocco importato.

**Risoluzione import:** 1) cerca nome nei registry; 2) hash candidato (TOFU o range); 3) se whitelist abilitata, verifica hash; 4) errore se non autorizzato o non raggiungibile.

**Verifiche a compile-time:**

*Dipendenze:*
```
Error: dipendenza mancante
  main.ldc:5  import: "os"
  → os@blake3:abc123 richiede crypto@blake3:def456
  Suggerimento: aggiungi `import: "crypto";` in capabilities
```

*Requisiti:*
```
Error: requisito non soddisfatto
  main.ldc:1  env.ram = 2048
  → os@blake3:abc123 richiede ram >= 4096
  Suggerimento: usa un target con più RAM, o rimuovi `import: "os";`
    se non ti serve
```

*Operazioni:*
```
Error: operazione non disponibile
  spawn_process richiede il blocco "os"
  Suggerimento: aggiungi `import: "os";` in capabilities
```

*Target:*
```
Error: target non compatibile
  os@blake3:abc123 non supporta arduino_uno
  Suggerimento: rimuovi `import: "os";` per questo target, o compila
    per uno dei target supportati (`laplace info os` li elenca)
```

**Conflitti:**

*Nome (funzioni):*
```
Error: nome ambiguo
  spawn_process fornito da os@... e process@...
  Suggerimento: os::spawn_process(...)
```

*Tipo opaco:* stessa regola dei nomi di funzione in conflitto — un nome di tipo ambiguo tra blocchi va disambiguato con `blocco::tipo` o `blocco@blake3:hash::tipo`; l'uso non qualificato in caso di conflitto è un errore di compilazione.

**Un'unica sintassi di disambiguazione**, `blocco::nome` (o, per una versione precisa, `blocco@blake3:hash::nome`), vale identica per funzioni, tipi opachi **e** costanti (§I.11) — non sono tre regole distinte, sono lo stesso meccanismo applicato a tre categorie di nomi importati.

*Versione:* versioni diverse coesistono nello store, namespace `blocco@blake3:hash::funzione`.

*Capability:* non conflitto, **unione**. Runtime abilita se almeno un blocco richiede. Compilatore mostra lista finale:
```
Permessi runtime finali:
  seccomp: clone, execve, mount, umount2, kill, socket, connect
  capabilities: CAP_SYS_ADMIN, CAP_SYS_PTRACE, CAP_NET_RAW
```

**`laplace info os`** mostra contratto completo: nome, versione, tipo, hash, autore, licenza, dipendenze, forniti, permessi, requisiti, compatibilità target.

## 1. Hash, Firma e Trust

**Hash obbligatorio, firma opzionale, TOFU default.**

**Content-addressed.** Identità = hash, non nome. Hash calcolato a publish, scritto in `blockgen.toml`:
```
hash = BLAKE3(
    contenuto block.ldc +
    sorgenti +
    versione compilatore +
    opzioni build +
    dipendenze risolte (hash ricorsivi) +
    target
)
```
Ogni componente è separata da un byte `\0` prima dell'hashing (non concatenazione diretta di stringhe, per evitare ambiguità tipo `"ab"+"c"` vs `"a"+"bc"`); `versione compilatore` e `opzioni build` sono serializzate come stringa canonica (ordine fisso dei campi, sempre lo stesso formato). Modello Nix: cambia una virgola, cambia hash. Due blocchi con stesso nome ma hash diverso coesistono. Risolve conflitti di versione.

`hash_algorithm` accetta uno dei valori della tabella sopra (`blake3`, `sha256`, `sha512`; `sha1` rifiutato).

**Algoritmi:**
| Algoritmo | Dimensione | Sicurezza | Velocità | Uso |
|---|---|---|---|---|
| **BLAKE3** | 32 byte | Alta | Molto alta | Default |
| SHA-256 | 32 byte | Alta | Media | Compatibilità |
| SHA-512 | 64 byte | Alta | Media | Paranoia |
| SHA-1 | 20 byte | **Rotta** | Alta | Deprecato |

SHA-1 rifiutato dal compilatore.

**TOFU:** prima volta salva hash in `~/.laplace/trusted_hashes` — una riga per blocco, formato `nome@versione = blake3:hash`. Poi verifica. Se cambia:
```
Attenzione: os è cambiato
  Vecchio hash: blake3:abc123...
  Nuovo hash:   blake3:def456...
  Continuare? [y/N]
```
Leggero: no PKI, no CA, no blockchain.

**Firma opzionale:** in `security { }` di `main.ldc`:
```text
security {
    require_signature = true;
    trusted_keys = ["ed25519:abc123...", "ed25519:def456..."];
    signature_algorithm = "ed25519";
}
```
Ed25519 default, RSA-2048/4096 alternative. Default `require_signature = false`.

**Registry federato:**
| Backend | Pro | Contro |
|---|---|---|
| **Repo Git** | Gratis, versionato, federato | Lento, no search |
| **HTTP** | Veloce, search | Centralizzato |
| **IPFS** | Decentralizzato | Lento, poco diffuso |

Default: repo Git. Configurabile in `security { }`:
```text
security {
    registry_url = "https://github.com/ldc-blocks/registry";
    registry_priority = "first";   // "first" (usa questo registry per primo) | "last" (fallback)
}
```
`trust_model` (§III.3) accetta `"tofu"` (default), `"strict"` (richiede sempre firma verificata, mai TOFU silenzioso), `"permissive"` (accetta hash sconosciuti senza chiedere conferma — sconsigliato fuori da ambienti di sviluppo chiusi).

## 2. Registry Laplace

`laplace` è compilatore, package manager, registry client. Ogni comando (non solo `build`) rispetta gli stessi flag di formato/accessibilità di §II.14: testo semplice di default, `--format=json` per output strutturato, `--lang`/`--quiet`/`--verbose` applicati uniformemente.

**Comandi:**
```bash
laplace build main.ldc
laplace block search os
laplace block info os
laplace block install os
laplace block remove os
laplace block list
laplace block update
laplace block verify os
laplace block sign os
laplace block publish os          # genera blockgen.toml
laplace security edit             # modifica blocco security{} (password OS)
laplace security log
laplace accessibility show        # legge ~/.laplace/accessibility.toml
laplace accessibility set <k>=<v>
laplace accessibility edit        # nessuna password richiesta
```

**Store content-addressed**, naming `<algoritmo>-<hash>-<nome>-<versione>/`:
```
~/.laplace/store/
├── blake3-abc123-os-1.0.0/
│   ├── blockgen.toml
│   ├── os-1.0.0-linux-x86_64.so
│   └── src/
└── blake3-def456-crypto-1.0.0/
```
Due versioni coesistono. `remove` solo se nessun programma lo usa.

**Cross-compilation:** registry fornisce binari pre-compilati, sorgenti, build automatica. Naming:
```
<nome>-<versione>-<os>-<arch>.<ext>
os-1.0.0-linux-x86_64.so
os-1.0.0-arduino_uno.hex
```

## 3. Autogenerazione — `ldc.toml`

*(Questa sezione ripete e completa quanto introdotto in §I.3: la ridondanza è voluta — §I.3 introduce `ldc.toml` dal punto di vista del linguaggio, questa sezione lo completa dal punto di vista dell'ecosistema/blocchi. Non è testo da unificare.)*

**Modello a 2 file totali per progetto:**
- **`main.ldc`** — scritto dall'utente. Contiene TUTTO: hardware, dipendenze, sicurezza, logica. Nessun TOML separato.
- **`ldc.toml`** — autogenerato da Laplace ad ogni build. Non modificabile a mano.

**📌 La fusione non tocca le garanzie di `[security_rule]`.** Il blocco `security { }` di `main.ldc` viene estratto in `[security_rule]` di `ldc.toml`. Protetta da password OS, nessun codice/`listener` può modificarla a runtime, ogni scrittura loggata. Preservata ad ogni rigenerazione, non riscritta. Modificabile solo via `laplace security edit` (password OS), che riscrive il blocco `security { }` in `main.ldc`.

**Esempio `ldc.toml` generato:**
```toml
[meta]
generated_by = "laplace 0.1.0"
generated_at = "2026-09-11T15:00:00Z"
source = "main.ldc"
source_hash = "blake3:abc123..."

[project]
name = "scooter_controller"
version = "0.1.0"

[blocks]
os = { version = "1.0.0", hash = "blake3:aaa111...", path = "~/.laplace/store/blake3-aaa111-os-1.0.0/" }

[integrity]
hash_algorithm = "blake3"
require_signature = false
trust_model = "tofu"
signature_algorithm = "ed25519"

[registry]
url = "https://blocks.ldc.dev"
priority = "first"

[runtime]
seccomp_enabled = true
seccomp_default = "deny"
capabilities_enabled = true
capabilities_default = "deny"
namespaces_enabled = true
cgroups_enabled = true

[crypto]
encryption_default = "e2e_ecc"
key_rotation_days = 30

[log]
level = "info"

[security_rule]
max_hardware_speed = 30
max_hardware_temp = 85
can_modify_max_speed = false
can_disable_watchdog = false
log_all_rule_changes = true
log_path = "./logs/security.log"
auto_lock_after_crash = 3
reset_speed_after_crash = true
bridge_keys = "injected"
authorized_listeners = ["#battery_check"]

[security_rule.unsafe]
allow_unsafe_blocks = false
allow_raw_pointers = false
allow_inline_asm = false
unsafe_allowed_modules = ["./hardware_driver.ldc"]

[security_rule.isr]
allow_isr = false
isr_allowed_modules = ["./hardware_driver.ldc"]

[security_rule.limits]
chunk_ram_ratio = 0.6
max_string_multiplier = 0.5
max_recursion_depth = 32
max_delay_ms = 10000
eeprom_wear_leveling_slots = 8

[security_rule.blocks]
authorized = [
    "os@blake3:aaa111",
    "network@blake3:bbb222",
    "sensors_lib@blake3:ccc333"
]
whitelist_enabled = true
```

**Se `security { }` non è presente in `main.ldc`**, Laplace usa default sicuri e li scrive in `ldc.toml` con un warning:
```
Warning: nessun blocco security{} trovato in main.ldc
  Uso default sicuri per tutte le decisioni di sicurezza.
  Suggerimento: aggiungi un blocco security{} in main.ldc per
    fissare i limiti hardware.
```

**Dove si trova `ldc.toml`:** accanto a `main.ldc`. Rigenerato ad ogni build. Se cancellato, ricreato con default. Se modificato a mano tra una build e l'altra: la modifica viene **rilevata e sovrascritta** alla build successiva (Laplace confronta `source_hash` con l'hash corrente di `main.ldc` e rigenera da zero se non corrispondono, o comunque ad ogni build esplicita), con un warning loggato — `ldc.toml` non è mai una fonte di verità, solo `main.ldc` lo è.

**Modello a 2 file totali:**

| File | Chi lo scrive | Modificabile | Versionato |
|---|---|---|---|
| `main.ldc` | Utente | Sì | Sì |
| `ldc.toml` | Laplace | No (tranne `security{}` via tooling) | No (o sì per build riproducibili) |

**Nessun TOML scritto dall'utente nel progetto. Nessun `project.toml`. Nessun `overrides.toml`. Nessun `block.toml`.** (`~/.laplace/accessibility.toml`, §II.14, è l'unica eccezione voluta: vive fuori dal progetto, per costruzione.)

**Profili opzionali in `main.ldc`:**
```text
security {
    profile: "paranoid";   // paranoid | standard | permissive
}
```

Se `profile` è omesso, il default è `standard`.

| Decisione | paranoid | standard | permissive |
|---|---|---|---|
| `hash_algorithm` | blake3 | blake3 | sha256 |
| `require_signature` | true | false | false |
| `trust_model` | strict | tofu | permissive |
| `seccomp_enabled` | true | true | false |
| `seccomp_default` | deny | deny | allow |
| `log_level` | debug | info | warn |

L'utente può sovrascrivere singole decisioni in `security { }`. Il blocco **non è profilato** per intero: il profilo è un default, i campi espliciti vincono.

**Warning** se si devia da default sicuro:
```
Warning: hash_algorithm = "sha256"
  BLAKE3 è più veloce e altrettanto sicuro.
```

**Decisioni rotte rifiutate:**
```
Error: hash_algorithm = "sha1"
  SHA-1 è deprecato e rotto.
```

## 4. ABI e Binari Stabili

**📌 Decisione:** ABI definita come formato binario esatto. Compilatore sceglie versione solo se developer la fissa in `build { }`, altrimenti default (nessun rallentamento).

**Versioning:** blocco dichiara ABI in `build { abi: "1.0"; }` di `block.ldc`. Compiler verifica compatibilità:
```
Error: ABI incompatibile
  os@blake3:abc123 ha ABI 1.0, ma il compilatore richiede ABI 2.0
```

**Header binario:**
```
Offset  Size  Campo
0x00    4     Magic: "LDC\0"
0x04    2     ABI major (u16 LE)
0x06    2     ABI minor (u16 LE)
0x08    1     Endianness: 0x01 little, 0x02 big
0x09    1     Arch: 0x01 x86_64, 0x02 arm64, 0x03 riscv64, 0x04 avr, 0xFF generic
0x0A    2     Reserved
0x0C    4     Total size (u32)
0x10    32    BLAKE3 hash payload
0x30    4     Num exported functions (u32)
0x34    4     Num exported types (u32)
0x38    4     Num exported constants (u32)
0x3C    4     Offset to function table
0x40    4     Offset to type table
0x44    4     Offset to constant table
0x48    4     Offset to string table
0x4C    4     Offset to payload start
0x50-0x5F  Reserved
0x60    ...   Inizio tabelle
```

**Tabella funzioni** (record 24 byte):
```
0x00  4  Name offset into string table
0x04  4  Name length
0x08  4  Function pointer offset (dal payload)
0x0C  1  Num params (u8)
0x0D  1  Return type code
0x0E  2  Reserved
0x10  8  Param type codes (8 u8, oltre 8 → array esteso)
```

**Codici tipo:**
```
0x00 void
0x01 8bit
0x02 16bit
0x03 32bit
0x04 32bit.float
0x05 bool
0x06 string
0x07 ptr (raw)
0x08 opaque (u32 index type table)
0x09 array  (u32 type_code + u32 max)
0xFF invalid
```

**Tabella tipi** (record 12 byte):
```
0x00  4  Name offset
0x04  4  Name length
0x08  2  Size in bit
0x0A  1  Kind: 0x01 opaque, 0x02 enum, 0x03 struct
0x0B  1  Reserved
```

**Tabella costanti** (record 16 byte):
```
0x00  4  Name offset
0x04  4  Name length
0x08  1  Type code
0x09  1  Reserved
0x0A  2  Reserved
0x0C  4  Value (u32)
```

**Note di lettura sulle tabelle sopra:**
- **Arch** (header binario): l'elenco (`x86_64`, `arm64`, `riscv64`, `avr`, `generic`) non è chiuso — nuovi valori possono essere aggiunti in future minor ABI senza rompere il parsing (`generic`/`0xFF` resta il fallback per architetture non ancora enumerate).
- **Return type code** nella tabella funzioni: se la funzione ritorna `void`, il campo vale `0x00` (lo stesso codice tipo `void` della tabella "Codici tipo").
- **Param type codes oltre 8**: se `Num params > 8`, gli 8 slot fissi del record contengono i primi 8, e un puntatore/offset ai restanti (l'"array esteso" citato) segue immediatamente dopo il record a lunghezza fissa, prima del record della funzione successiva.
- **Kind** nella tabella tipi (`opaque`/`enum`/`struct`): elenco anch'esso aperto a estensione futura, stesso principio di **Arch**.
- **Type code** nella tabella costanti: una costante è sempre un tipo **scalare built-in** (numerico o bool) — mai `array`, `opaque` o `string` lunga, che non hanno una rappresentazione a lunghezza fissa adatta al campo `Value` da 4 byte; una costante stringa (rara) userebbe `Value` come offset nella string table invece che come valore diretto.
- **Value (u32)**: per tipi più piccoli di 32 bit il valore è esteso con zeri nei byte alti; non esistono in 0.1 costanti scalari più grandi di 32 bit.

**String table:** blob UTF-8, tutti i nomi concatenati. Offset relativi all'inizio.

**Payload:** codice compilato. Non ispezionabile. Compiler salta a `Function pointer offset`.

**Allineamento struct esportate:** allineamento naturale per-target — ogni campo allineato alla propria dimensione (1/2/4 byte), padding inserito dove serve, dimensione totale della struct arrotondata al multiplo dell'allineamento del campo più grande. Su **AVR** l'allineamento naturale è **1 byte** per ogni campo (nessun padding: è la convenzione ABI-C standard di quell'architettura). Stessa convenzione ABI-C del target di destinazione (il layout che avrebbe la struct equivalente in C su quell'architettura): compatibilità diretta con `provides.types` senza bisogno di uno schema di layout separato.

**Chiamata:** cdecl-like LDC — stessa convenzione del cdecl C: i parametri sono impilati **da destra a sinistra** (l'ultimo parametro è il primo pushato), allineati a 8 byte (o 4 su 32-bit), ed è il **chiamante** (caller) a ripulire lo stack dopo la chiamata, non il chiamato. Ritorno in registro (`rax`, `x0`) per ≤32 bit; altrimenti **sret**: il chiamante alloca lo spazio di ritorno e ne passa l'indirizzo come **primo parametro implicito**, prima di tutti i parametri dichiarati. Little-endian su tutti i target 1.0 (AVR big-endian storico, convertito al confine: ogni valore multi-byte che attraversa la function table dell'ABI viene byte-swappato in ingresso e uscita dal codice generato per quel target, in modo che il payload binario resti little-endian indipendentemente dal target che lo consuma). Nessuna eccezione attraverso confine: errori via `result` o tipo opaco equivalente — `result` non è un tipo built-in del linguaggio, è una convenzione: ogni blocco che può fallire dichiara un proprio tipo opaco (tipicamente un `8bit` con un set di codici, come `result` nell'esempio `os` di §III.0) e lo usa come tipo di ritorno delle funzioni che possono fallire, invece di lanciare un'eccezione attraverso il confine ABI.

**Sorgenti per audit:** binari nel registry, sorgenti pure. Ricompila e verifica hash. Modello "trust, but verify".
```
Warning: hash mismatch
  os@blake3:abc123 (binario) != blake3:xyz789 (ricompilato)
  Suggerimento: il binario nel registry non corrisponde ai sorgenti;
    usa `laplace block verify os` per investigare, o scarica di nuovo
    il blocco con `laplace block install os --force`
```

**Compatibilità:** stessa major, minor diversa: compatibile, warning se minor più vecchia. Major diversa: incompatibile, errore. Arch diversa: binario per-arch, `--target` sceglie.

**Niente ABI multiple nello stesso processo.** Un programma carica una sola ABI major. Blocchi con major diverse non coesistono. Compilatore segnala errore.
