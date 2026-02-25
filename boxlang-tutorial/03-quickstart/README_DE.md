# BoxLang Schnellstart

Diese Anleitung hilft Ihnen, Ihr erstes BoxLang-Programm in 5 Minuten zu erstellen und auszuführen.

## Neues Projekt erstellen

### Mit dem Befehl boxlang new

```bash
# Neues Projekt mit dem Namen hello_boxlang erstellen
boxlang new hello_boxlang

# In das Projektverzeichnis wechseln
cd hello_boxlang
```

### Projektstruktur

Nach der Erstellung sehen Sie folgende Verzeichnisstruktur:

```
hello_boxlang/
├── box.toml          # Projektkonfigurationsdatei
├── README.md         # Projektbeschreibung
├── .gitignore        # Git Ignore-Datei
└── src/
    └── main.box      # Hauptprogramm-Einstieg
```

## Erstes Programm schreiben

### Standard main.box

```boxlang
module hello_boxlang;

pub fn main() {
    println("Hello, BoxLang!");
}
```

### Benutzerdefiniertes Programm

Lassen Sie uns das Programm modifizieren, um einige interaktive Funktionen hinzuzufügen:

```boxlang
module hello_boxlang;

pub fn main() {
    // Willkommensnachricht ausgeben
    println("Willkommen bei BoxLang!");
    println("==================");
    
    // Variablendeklaration
    let name = "BoxLang Entwickler";
    let version = 1.0;
    
    // String-Formatierung ausgeben
    println("Hallo, {name}!");
    println("Aktuelle Version: {version}");
    
    // Funktion aufrufen
    let result = add(10, 20);
    println("10 + 20 = {result}");
}

// Additionsfunktion definieren
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
```

## Kompilieren und Ausführen

### Methode 1: Bauen und dann Ausführen

```bash
# Projekt kompilieren
boxlang build

# Kompiliertes Programm ausführen
boxlang run
```

### Methode 2: Direkt Ausführen (Entwicklungsmodus)

```bash
# Sofort kompilieren und ausführen
boxlang run --dev
```

### Methode 3: Ausgabedatei angeben

```bash
# Zu einem bestimmten ausführbaren Namen kompilieren
boxlang compile src/main.box -o myprogram

# Direkt ausführen
./myprogram
```

## Entwicklungsworkflow

### 1. Code Bearbeiten
Verwenden Sie Ihren bevorzugten Editor, um die Datei `src/main.box` zu modifizieren.

### 2. Syntax Prüfen
```bash
# Codesyntax ohne Kompilierung prüfen
boxlang check
```

### 3. Projekt Bauen
```bash
# Entwicklungsmodus (schnelle Kompilierung, keine Optimierung)
boxlang build --dev

# Release-Modus (optimierte Kompilierung)
boxlang build --release
```

### 4. Tests Ausführen
```bash
# Tests im Projekt ausführen
boxlang test
```

## Häufige Befehle - Spickzettel

| Befehl | Beschreibung |
|--------|--------------|
| `boxlang new <name>` | Neues Projekt erstellen |
| `boxlang build` | Projekt bauen |
| `boxlang run` | Projekt ausführen |
| `boxlang check` | Syntaxprüfung |
| `boxlang test` | Tests ausführen |
| `boxlang clean` | Build-Dateien bereinigen |
| `boxlang fmt` | Code formatieren |

## Nächste Schritte

- [Grundlegende Syntax](../04-basic-syntax/README_DE.md) - BoxLang-Kernsyntax lernen
- [Projektstruktur](../05-project-structure/README_DE.md) - Projektkonfiguration und -struktur verstehen
