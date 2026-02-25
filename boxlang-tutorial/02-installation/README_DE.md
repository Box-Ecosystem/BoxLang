# BoxLang Installationsanleitung

## Systemanforderungen

### Mindestanforderungen
- **Betriebssystem**: Windows 10/11 (64-bit)
- **RAM**: 4 GB
- **Speicherplatz**: 2 GB verfügbar
- **Netzwerk**: Internetverbindung (für das Herunterladen von Abhängigkeiten)

### Empfohlene Konfiguration
- **Betriebssystem**: Windows 11 (64-bit)
- **RAM**: 8 GB
- **Speicherplatz**: 5 GB verfügbar
- **IDE**: Visual Studio 2022 oder VS Code

## Voraussetzungen

Bevor Sie BoxLang installieren, stellen Sie sicher, dass die folgende Software installiert ist:

### 1. Git
```bash
# Prüfen, ob installiert
git --version

# Download: https://git-scm.com/download/win
```

### 2. Rust Toolchain
```bash
# Installation mit rustup (empfohlen)
# Download: https://rustup.rs/

# Installation überprüfen
rustc --version
cargo --version
```

### 3. LLVM (Optional, für optimierte Kompilierung)
```bash
# Download: https://releases.llvm.org/download.html
# Empfohlene Version: 15.0 oder höher
```

## Installationsschritte

### Methode 1: Aus dem Quellcode bauen

#### 1. Repository klonen
```bash
git clone https://github.com/yourusername/box-ecosystem.git
cd box-ecosystem
```

#### 2. Compiler bauen
```bash
cd boxlang/compiler
cargo build --release
```

Nach der Kompilierung befindet sich die ausführbare Datei unter:
```
target/release/boxlang.exe
```

#### 3. Zum System PATH hinzufügen
```powershell
# PowerShell (als Administrator ausführen)
[Environment]::SetEnvironmentVariable(
    "Path",
    [Environment]::GetEnvironmentVariable("Path", "User") + ";C:\path\to\box-ecosystem\boxlang\compiler\target\release",
    "User"
)
```

### Methode 2: Mit Installationsskript (Empfohlen)

```powershell
# PowerShell
irm https://boxlang.dev/install.ps1 | iex
```

### Methode 3: Vorkompilierte Version herunterladen

1. Besuchen Sie [GitHub Releases](https://github.com/yourusername/box-ecosystem/releases)
2. Laden Sie die neueste `boxlang-windows-x64.zip` herunter
3. Entpacken Sie in das gewünschte Verzeichnis
4. Fügen Sie das Verzeichnis zum System PATH hinzu

## Installation überprüfen

```bash
# Version prüfen
boxlang --version

# Hilfe anzeigen
boxlang --help

# Compiler testen
boxlang doctor
```

## Entwicklungsumgebung konfigurieren

### VS Code Einrichtung

#### 1. Erweiterungen installieren
- BoxLang Language Support
- BoxLang Debugger

#### 2. settings.json konfigurieren
```json
{
    "boxlang.compilerPath": "C:\\path\\to\\boxlang.exe",
    "boxlang.enableLinter": true,
    "boxlang.formatOnSave": true
}
```

### Visual Studio Einrichtung

#### 1. BoxLang VS Extension installieren
```bash
boxlang install-vs-extension
```

#### 2. Projekteigenschaften konfigurieren
- Rechtsklick auf Projekt → Eigenschaften → BoxLang
- Compiler-Pfad und Build-Optionen festlegen

## FAQ

### Q: "linker not found" Fehler während der Kompilierung
**A**: Visual C++ Build Tools installieren
```powershell
# Installation über Visual Studio Installer
# Oder eigenständige Version herunterladen
https://visualstudio.microsoft.com/visual-cpp-build-tools/
```

### Q: cargo build schlägt mit fehlenden Abhängigkeiten fehl
**A**: Rust Toolchain aktualisieren
```bash
rustup update
rustup component add rust-src
```

### Q: boxlang Befehl wird nicht erkannt
**A**: PATH Konfiguration prüfen
```powershell
# Aktuellen PATH anzeigen
$env:Path -split ";"

# Überprüfen, ob das Verzeichnis mit boxlang.exe enthalten ist
```

## BoxLang aktualisieren

```bash
# Aus dem Quellcode aktualisieren
cd box-ecosystem
git pull
cd boxlang/compiler
cargo build --release

# Oder mit Update-Befehl
boxlang self-update
```

## BoxLang deinstallieren

```powershell
# Installationsverzeichnis entfernen
Remove-Item -Recurse -Force "C:\path\to\box-ecosystem"

# Aus PATH entfernen
# Systemumgebungsvariablen manuell bearbeiten
```

## Nächste Schritte

- [Schnellstart](../03-quickstart/README_DE.md) - Erstellen Sie Ihr erstes BoxLang-Projekt
