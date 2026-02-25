# BoxLang Projektstruktur

Dieses Kapitel stellt die Standardstruktur und Konfiguration von BoxLang-Projekten vor.

## Standard Projektstruktur

Eine typische BoxLang-Projektstruktur sieht wie folgt aus:

```
myproject/
├── box.toml              # Projektkonfigurationsdatei (erforderlich)
├── README.md             # Projektdokumentation
├── LICENSE               # Lizenzdatei
├── .gitignore            # Git Ignore-Datei
├── .boxlang/             # BoxLang internes Verzeichnis
│   └── cache/            # Kompilierungs-Cache
├── src/                  # Quellcode-Verzeichnis
│   ├── main.box          # Hauptprogramm-Einstieg (ausführbares Projekt)
│   ├── lib.box           # Bibliotheks-Einstieg (Bibliotheksprojekt)
│   └── utils/            # Untermodul-Verzeichnis
│       └── helper.box
├── tests/                # Testcode
│   └── integration_test.box
├── examples/             # Beispielcode
│   └── basic_usage.box
├── docs/                 # Projektdokumentation
│   └── api.md
└── target/               # Build-Ausgabeverzeichnis
    ├── debug/            # Debug-Build-Ausgabe
    └── release/          # Release-Build-Ausgabe
```

## box.toml Konfiguration

### Grundkonfiguration

```toml
[package]
name = "myproject"           # Projektname
version = "1.0.0"            # Version (folgt semantischer Versionierung)
authors = ["Your Name <you@example.com>"]
edition = "2024"             # BoxLang Version
license = "MIT"
description = "Projektbeschreibung"
repository = "https://github.com/username/myproject"

[dependencies]
# Abhängigkeiten
std = { version = "1.0" }
serde = { version = "0.8", features = ["derive"] }

[dependencies.mylib]
path = "../mylib"            # Lokale Pfadabhängigkeit

[dependencies.remote-lib]
git = "https://github.com/user/repo.git"
branch = "main"

[build]
target = "x86_64-pc-windows-msvc"  # Zielplattform
opt-level = 3                      # Optimierungsstufe (0-3)
debug = false                      # Debug-Info einschließen

[features]
default = ["std"]
std = []
no_std = []
embedded = ["no_std"]
```

### Multi-Target Konfiguration

```toml
# Windows Desktop-App
[[target]]
name = "windows-app"
target = "x86_64-pc-windows-msvc"
output-type = "exe"

# zetboxos Embedded-App
[[target]]
name = "zetboxos-app"
target = "thumbv7em-none-eabihf"
output-type = "bin"
```

## Quellcode-Organisation

### Ausführbares Projekt

```boxlang
// src/main.box
module myproject;

use std::io;
use utils::helper;

pub fn main() {
    println("Programm gestartet");
    helper::do_something();
}
```

### Bibliotheksprojekt

```boxlang
// src/lib.box
module mylib;

pub mod core;
pub mod utils;

// Öffentliche API
pub use core::engine::Engine;
pub use utils::helpers::format_data;
```

### Untermodule

```boxlang
// src/utils/helper.box
module myproject::utils::helper;

pub fn do_something() {
    println("Etwas tun...");
}

pub struct Helper {
    name: str,
}

impl Helper {
    pub fn new(name: str) -> Helper {
        Helper { name }
    }
}
```

## Modulsystem

### Moduldeklaration

```boxlang
// Aktuelles Modul deklarieren
module myproject::core::engine;

// Standardbibliothek importieren
use std::collections::HashMap;
use std::io::{File, Read};

// Lokale Module importieren
use crate::utils::helper;
use super::config::Config;

// Öffentlicher Re-Export
pub use self::types::EngineType;
```

### Modulsichtbarkeit

```boxlang
// Standardmäßig privat
fn private_function() {}
struct PrivateStruct {}

// Öffentlich
pub fn public_function() {}
pub struct PublicStruct {}

// Nur crate-sichtbar
pub(crate) fn crate_function() {}

// Nur Elternmodul-sichtbar
pub(super) fn parent_visible() {}
```

## Testorganisation

### Unit-Tests

```boxlang
// src/calculator.box
module myproject::calculator;

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Eingebaute Tests
#[test]
fn test_add() {
    assert_eq!(add(2, 3), 5);
    assert_eq!(add(-1, 1), 0);
}

#[test]
#[should_panic]
fn test_overflow() {
    // Überlaufbedingung testen
}
```

### Integrationstests

```boxlang
// tests/integration_test.box
use myproject::calculator;

#[test]
fn test_calculator_integration() {
    let result = calculator::add(10, 20);
    assert_eq!(result, 30);
}
```

## Workspace

### Workspace-Konfiguration

```toml
# box.toml (Workspace-Root)
[workspace]
members = [
    "mylib",
    "myapp",
    "utils",
]

[workspace.dependencies]
serde = "1.0"
```

### Workspace-Struktur

```
workspace/
├── box.toml              # Workspace-Konfiguration
├── mylib/
│   ├── box.toml
│   └── src/
├── myapp/
│   ├── box.toml
│   └── src/
└── utils/
    ├── box.toml
    └── src/
```

## Build-Konfiguration

### Bedingte Kompilierung

```boxlang
// Plattformspezifischer Code
#[cfg(target_os = "windows")]
fn platform_specific() {
    // Windows-Code
}

#[cfg(target_os = "zetboxos")]
fn platform_specific() {
    // zetboxos-Code
}

// Feature-Flags
#[cfg(feature = "std")]
use std::collections::HashMap;

#[cfg(not(feature = "std"))]
use alloc::collections::HashMap;
```

### Build-Skripte

```toml
# box.toml
[package]
build = "build.box"
```

```boxlang
// build.box
use std::process::Command;

fn main() {
    // Code generieren
    println!("cargo:rerun-if-changed=src/schema.json");
    
    // Umgebungsvariablen setzen
    println!("cargo:rustc-env=BUILD_TIME=2024-01-01");
}
```

## Nächste Schritte

- [AppBox Verpackung](../06-appbox-packaging/README_DE.md) - Lernen Sie, wie Sie Anwendungen verpacken und veröffentlichen
