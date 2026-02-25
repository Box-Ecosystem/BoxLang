# BoxLang Grundlegende Syntax

Dieses Kapitel stellt die Kernsyntax-Konzepte von BoxLang vor.

## Variablen und Konstanten

### Variablendeklaration

```boxlang
// Unveränderliche Variable (Standard)
let x = 10;
// x = 20;  // Fehler! Unveränderliche Variablen können nicht neu zugewiesen werden

// Veränderliche Variable
let mut y = 20;
y = 30;  // OK

// Typannotation (optional)
let count: i32 = 100;
let price: f64 = 19.99;
let name: str = "BoxLang";
```

### Konstanten

```boxlang
// Konstanten müssen zur Kompilierzeit bestimmt werden
const PI: f64 = 3.14159;
const MAX_SIZE: i32 = 100;
```

## Datentypen

### Grundtypen

```boxlang
// Ganzzahlen
let a: i8 = 127;        // 8-Bit vorzeichenbehaftete Ganzzahl
let b: i16 = 32767;     // 16-Bit vorzeichenbehaftete Ganzzahl
let c: i32 = 2147483647; // 32-Bit vorzeichenbehaftete Ganzzahl (Standard)
let d: i64 = 9223372036854775807; // 64-Bit vorzeichenbehaftete Ganzzahl

// Vorzeichenlose Ganzzahlen
let e: u32 = 4294967295;

// Gleitkommazahlen
let f: f32 = 3.14;      // 32-Bit Gleitkomma
let g: f64 = 3.14159;   // 64-Bit Gleitkomma (Standard)

// Boolesch
let flag: bool = true;

// Zeichen
let ch: char = 'A';

// Zeichenkette
let s: str = "Hello, BoxLang!";
```

### Zusammengesetzte Typen

```boxlang
// Array
let arr = [1, 2, 3, 4, 5];
let first = arr[0];  // Elementzugriff

// Tupel
let tuple = (1, "hello", 3.14);
let num = tuple.0;   // Erstes Element
let text = tuple.1;  // Zweites Element
```

## Funktionen

### Grundlegende Funktionsdefinition

```boxlang
// Funktion ohne Rückgabewert
fn greet(name: str) {
    println("Hello, {name}!");
}

// Funktion mit Rückgabewert
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// Ausdrucksrückgabe (return weglassen)
fn multiply(a: i32, b: i32) -> i32 {
    a * b  // Letzter Ausdruck ist der Rückgabewert
}
```

### Funktionsüberladung

```boxlang
fn process(x: i32) -> i32 {
    x * 2
}

fn process(x: str) -> str {
    x + " processed"
}
```

## Kontrollfluss

### Bedingte Anweisungen

```boxlang
let number = 10;

// if-else
if number > 0 {
    println("Positiv");
} else if number < 0 {
    println("Negativ");
} else {
    println("Null");
}

// if als Ausdruck
let result = if number > 5 { "Groß" } else { "Klein" };
```

### Schleifen

```boxlang
// while Schleife
let mut i = 0;
while i < 5 {
    println("i = {i}");
    i = i + 1;
}

// for Schleife (Bereich)
for j in 0..5 {
    println("j = {j}");
}

// for Schleife (Array)
let arr = [10, 20, 30];
for item in arr {
    println("item = {item}");
}

// loop Endlosschleife
let mut counter = 0;
loop {
    counter = counter + 1;
    if counter >= 10 {
        break;
    }
}
```

## Strukturen

### Strukturen definieren

```boxlang
pub struct Point {
    x: f64,
    y: f64,
}

pub struct User {
    name: str,
    age: u32,
    email: str,
}
```

### Strukturen verwenden

```boxlang
// Instanz erstellen
let p = Point { x: 10.0, y: 20.0 };
let user = User {
    name: "Alice",
    age: 30,
    email: "alice@example.com",
};

// Felder zugriff
println("x = {p.x}, y = {p.y}");
println("Name: {user.name}");

// Strukturmethoden
impl Point {
    // Methode
    fn distance_from_origin(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    
    // Assoziierte Funktion (Konstruktor)
    fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }
}
```

## Aufzählungen

### Grundlegende Aufzählung

```boxlang
pub enum Direction {
    North,
    South,
    East,
    West,
}

let dir = Direction::North;
```

### Aufzählung mit Daten

```boxlang
pub enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(str),
    ChangeColor(i32, i32, i32),
}

let msg1 = Message::Quit;
let msg2 = Message::Move { x: 10, y: 20 };
let msg3 = Message::Write("Hello".to_string());
```

### Option Typ

```boxlang
pub enum Option<T> {
    Some(T),
    None,
}

// Verwendungsbeispiel
let some_number = Option::Some(5);
let no_number: Option<i32> = Option::None;
```

## Pattern Matching

```boxlang
let msg = Message::Move { x: 10, y: 20 };

match msg {
    Message::Quit => {
        println("Beenden");
    }
    Message::Move { x, y } => {
        println("Bewegen zu ({x}, {y})");
    }
    Message::Write(text) => {
        println("Text: {text}");
    }
    Message::ChangeColor(r, g, b) => {
        println("Farbe: RGB({r}, {g}, {b})");
    }
}

// if let vereinfachtes Matching
if let Message::Write(text) = msg {
    println("Text empfangen: {text}");
}
```

## Modulsystem

```boxlang
// Modul deklarieren
module myproject;

// Andere Module importieren
use std::io;
use std::fs::File;

// Öffentliches Modul
pub mod utils;

// Elemente aus Modul verwenden
use utils::helper;
```

## Pipeline-Operator

BoxLang unterstützt den Pipeline-Operator `|>`, der Datenverarbeitungsketten klarer und lesbarer macht.

### Grundlegende Verwendung

Der Pipeline-Operator übergibt das Ergebnis des linken Ausdrucks als Argument an die Funktion auf der rechten Seite:

```boxlang
fn double(x: i32) -> i32 {
    return x * 2;
}

fn add_one(x: i32) -> i32 {
    return x + 1;
}

fn square(x: i32) -> i32 {
    return x * x;
}

pub fn main() {
    // Traditioneller Ansatz (verschachtelte Aufrufe)
    let result1 = square(add_one(double(5)));
    // Ausführungsreihenfolge: double(5) -> add_one(10) -> square(11) = 121
    
    // Pipeline-Operator Ansatz (klarer)
    let result2 = 5 |> double |> add_one |> square;
    // Gleiches Ergebnis: 121, aber Lesereihenfolge entspricht Ausführungsreihenfolge
}
```

### Funktionsweise

- `a |> f` ist äquivalent zu `f(a)`
- `a |> f |> g` ist äquivalent zu `g(f(a))`
- Daten fließen von links nach rechts, was der natürlichen Leserichtung entspricht

### Verkettete Pipeline

```boxlang
fn increment(x: i32) -> i32 {
    return x + 1;
}

fn double(x: i32) -> i32 {
    return x * 2;
}

fn square(x: i32) -> i32 {
    return x * x;
}

fn negate(x: i32) -> i32 {
    return -x;
}

pub fn main() {
    let x = 3;
    
    // Mehrstufige Pipeline
    let result1 = x |> increment |> double;
    // 3 -> 4 -> 8
    
    let result2 = x |> double |> square;
    // 3 -> 6 -> 36
    
    let result3 = x |> increment |> double |> square;
    // 3 -> 4 -> 8 -> 64
    
    let result4 = x |> double |> square |> negate;
    // 3 -> 6 -> 36 -> -36
}
```

### Mathematisches Pipeline-Beispiel

```boxlang
fn absolute(x: i32) -> i32 {
    if x < 0 {
        return -x;
    }
    return x;
}

fn power_of_two(x: i32) -> i32 {
    return x * x;
}

pub fn main() {
    let value1 = 5;
    let result1 = value1 |> power_of_two |> absolute;
    // 5 -> 25 -> 25
    
    let value2 = -3;
    let result2 = value2 |> absolute |> power_of_two;
    // -3 -> 3 -> 9
    
    let result3 = 2 |> power_of_two |> power_of_two;
    // 2 -> 4 -> 16
}
```

### Vorteile des Pipeline-Operators

1. **Lesbarkeit**: Die Lesereihenfolge des Codes entspricht der Ausführungsreihenfolge
2. **Wartbarkeit**: Einfaches Hinzufügen, Entfernen oder Neuordnen von Verarbeitungsschritten
3. **Reduzierte Verschachtelung**: Vermeidet tief verschachtelte Funktionsaufrufe
4. **Klarer Datenfluss**: Zeigt deutlich den Datentransformationsprozess

```boxlang
// Vergleich: Traditionelle Verschachtelung vs. Pipeline-Operator

// Traditioneller Ansatz (von innen nach außen lesen)
let result = format(process(validate(parse(data))));

// Pipeline-Ansatz (von links nach rechts lesen)
let result = data |> parse |> validate |> process |> format;
```

## Kommentare

```boxlang
// Einzeiliger Kommentar

/*
 * Mehrzeiliger Kommentar
 * Kann mehrere Zeilen umfassen
 */

/// Dokumentationskommentar (für Funktionen, Strukturen usw.)
pub fn documented_function() {
    //! Innerer Dokumentationskommentar
}
```

## Nächste Schritte

- [Projektstruktur](../05-project-structure/README_DE.md) - Erfahren Sie mehr über Projektkonfiguration und Modulorganisation
