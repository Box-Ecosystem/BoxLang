# BoxLang

BoxLang ist eine vielseitige systemnahe Programmiersprache, die fuer verschiedene Umgebungen entwickelt wurde und von modernen Sprachen wie Rust, Go und Zig inspiriert ist, bei der gleichzeitig einzigartige Eigenschaften und Syntax angeboten werden.

## Designziele

- **Windows First**: Native Unterstuetzung fuer Windows-Entwicklung
- **Embedded Friendly**: Entwickelt fuer nahtlose Ausfuehrung in eingebetteten Umgebungen
- **Hohe Leistung**: AOT-Kompilierung, Zero-Cost-Abstraktion
- **Einfach zu erlernen**: Einfachere Syntax als Rust
- **Moderne Funktionen**: Unterstuetzung fuer async/await, Generics, Pattern Matching
- **Paketierungsunterstuetzung**: Native Unterstuetzung fuer Anwendungsverpackung

## Schnellstart

### Installation

```bash
git clone https://github.com/Box-Ecosystem/BoxLang.git
cd boxlang/compiler
cargo build --release
```

### Neues Projekt erstellen

```bash
boxlang new myproject
cd myproject
```

### Erstes Programm schreiben

Erstellen Sie `src/main.box`:

```boxlang
module myproject;

pub fn main() {
    println("Hello, BoxLang!");
}
```

### Kompilieren und Ausfuehren

```bash
boxlang compile src/main.box -o hello
boxlang build
boxlang run
```

### Anwendung paketieren

```bash
boxlang package
boxlang package -o ./dist -n "myapp" -v "1.0.0"
```

## Grundsyntax

```boxlang
// Variablendeklaration
let x = 10;           // Unveraenderlich
let mut y = 20;       // Veraenderlich
const PI = 3.14159;   // Konstante

// Funktionsdefinition
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// Struktur
pub struct Point {
    x: f64,
    y: f64,
}

// Aufzaehlung
pub enum Option<T> {
    Some(T),
    None,
}
```

## Projektstruktur

```
myproject/
├── box.toml          # Projektkonfiguration
├── README.md         # Projektbeschreibung
├── .gitignore        # Git Ignore-Datei
└── src/
    ├── main.box      # Hauptprogramm
    └── lib.box       # Bibliothekscode
```

## Lizenz

BoxLang steht unter der MIT-Lizenz. Für den vollständigen Lizenztext siehe die [LICENSE](LICENSE) Datei.

## Marken

"BoxLang" ist ein projektspezifischer Name, der zur Identifizierung dieser Programmiersprache verwendet wird. Dieser Name ist kein eingetragenes Warenzeichen, es sei denn, dies wird anders angegeben.

## Beitragen

Wir freuen uns über Beiträge zu BoxLang! Um beizutragen, bitte:

1. Fork das Repository
2. Erstelle einen neuen Branch für deine Funktion oder Bugfix
3. Mach deine Änderungen
4. Sende einen Pull Request

### Contributor License Agreement (CLA)

Indem du zu diesem Projekt beiträgst, stimmst du den Bedingungen des Contributor License Agreement (CLA) zu. Diese Vereinbarung stellt sicher, dass deine Beiträge ordnungsgemäß lizenziert sind und dass die Projektbetreuer die notwendigen Rechte haben, um deine Arbeit in das Projekt aufzunehmen.

Stelle sicher, dass dein Code den Codierungsrichtlinien des Projekts folgt und alle Tests besteht.

## Haftungsausschluss

BoxLang ist ein Open-Source-Projekt unter aktiver Entwicklung. Während wir uns bemühen, eine zuverlässige und sichere Programmiersprache bereitzustellen, verwenden Sie sie auf eigene Gefahr. Die Projektbetreuer machen keine ausdrücklichen oder stillschweigenden Garantien hinsichtlich der Funktionalität oder Eignung von BoxLang für einen bestimmten Zweck.
