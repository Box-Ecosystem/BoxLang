# AppBox Verpackung und Veröffentlichung

Dieses Kapitel behandelt, wie BoxLang-Projekte als AppBox-Format verpackt und im Box Ecosystem veröffentlicht werden.

## Was ist AppBox?

AppBox ist das Anwendungspaketformat für das Box Ecosystem, ähnlich wie Paketformate anderer Plattformen:

| Plattform | Paketformat |
|-----------|-------------|
| Box Ecosystem | **.appbox** |
| Android | .apk / .aab |
| iOS | .ipa |
| Windows | .msi / .exe |

### AppBox Funktionen

- **Selbstständig**: Enthält alle Abhängigkeiten, die zum Ausführen der App benötigt werden
- **Signaturüberprüfung**: Unterstützt digitale Signaturen für Sicherheit
- **Versionsverwaltung**: Integrierte Versionskontrolle und Update-Mechanismus
- **Plattformübergreifend**: Unterstützt Windows und zetboxos

## Grundlegende Verpackung

### Schnelle Verpackung

```bash
# Im Projektstammverzeichnis ausführen
boxlang package

# Ausgabe: target/package/myproject-1.0.0.appbox
```

### Benutzerdefinierte Verpackung

```bash
# Ausgabeverzeichnis angeben
boxlang package -o ./dist

# Anwendungsnamen angeben
boxlang package -n "MyApp"

# Version angeben
boxlang package -v "2.0.0"

# Kombinierte Verwendung
boxlang package -o ./dist -n "MyApp" -v "1.0.0"
```

## Verpackungskonfiguration

### box.toml Konfiguration

```toml
[package]
name = "myapp"
version = "1.0.0"
description = "Meine BoxLang-Anwendung"
authors = ["Your Name"]

[appbox]
# Anzeigename der Anwendung
name = "My Awesome App"

# Anwendungssymbol
icon = "assets/icon.png"

# Startbildschirm
splash = "assets/splash.png"

# Anwendungskategorie
category = "Productivity"

# Unterstützte Architekturen
architectures = ["x86_64", "arm64"]

# Minimale Systemanforderungen
min-os-version = "10.0"

# Berechtigungsdeklarationen
permissions = [
    "network",
    "filesystem",
    "camera",
]

# Ressourcendateien
resources = [
    "assets/**/*",
    "config/*.json",
    "locales/**/*.lang",
]

# Ausgeschlossene Dateien
exclude = [
    "tests/**/*",
    "docs/**/*",
    "*.log",
]

[appbox.metadata]
# Zusätzliche Metadaten
keywords = ["productivity", "tools"]
homepage = "https://myapp.example.com"
```

## Anwendungssignatur

### Signierschlüssel generieren

```bash
# Entwicklerschlüssel generieren
boxlang keygen --developer

# Veröffentlicherschlüssel generieren
boxlang keygen --publisher

# Schlüsseldatei angeben
boxlang keygen -o ./keys/mykey.pem
```

### Anwendung signieren

```bash
# Mit Standardschlüssel signieren
boxlang package --sign

# Schlüsseldatei angeben
boxlang package --sign --key ./keys/release.pem

# Schlüsselpasswort angeben
boxlang package --sign --key ./keys/release.pem --password-file ./keys/pass.txt
```

## Multi-Plattform-Verpackung

### Windows Desktop-App

```bash
# Windows-App verpacken
boxlang package --target windows-x64

# Installer erstellen
boxlang package --target windows-x64 --installer msi
```

### zetboxos Embedded-App

```bash
# zetboxos-App verpacken
boxlang package --target zetboxos-arm64

# Für Embedded optimieren
boxlang package --target zetboxos-arm64 --opt-size
```

### Multi-Target-Verpackung

```bash
# Alle Targets verpacken
boxlang package --all-targets

# Ausgabeverzeichnisstruktur:
# target/package/
# ├── myapp-1.0.0-windows-x64.appbox
# ├── myapp-1.0.0-zetboxos-arm64.appbox
# └── myapp-1.0.0-zetboxos-armv7.appbox
```

## Anwendungsüberprüfung

### Paketintegrität überprüfen

```bash
# AppBox-Datei überprüfen
boxlang verify myapp-1.0.0.appbox

# Detaillierte Überprüfung
boxlang verify myapp-1.0.0.appbox --verbose
```

### Anwendungsinformationen prüfen

```bash
# Anwendungsinformationen anzeigen
boxlang info myapp-1.0.0.appbox

# Beispielausgabe:
# Name: My Awesome App
# Version: 1.0.0
# Author: Your Name
# Size: 2.5 MB
# Signature: Valid
# Permissions: network, filesystem
```

## Anwendungen veröffentlichen

### Lokale Installation

```bash
# AppBox installieren
boxlang install myapp-1.0.0.appbox

# Installationsort angeben
boxlang install myapp-1.0.0.appbox --prefix /opt/apps

# Neuinstallation erzwingen
boxlang install myapp-1.0.0.appbox --force
```

### Im Box Store veröffentlichen

```bash
# Bei Box Store anmelden
boxlang login

# Anwendung veröffentlichen
boxlang publish myapp-1.0.0.appbox

# Als Beta veröffentlichen
boxlang publish myapp-1.0.0.appbox --beta

# Als Alpha veröffentlichen
boxlang publish myapp-1.0.0.appbox --alpha
```

### Private Registry-Veröffentlichung

```bash
# In private Registry veröffentlichen
boxlang publish myapp-1.0.0.appbox --registry https://private.registry.com

# API-Schlüssel verwenden
boxlang publish myapp-1.0.0.appbox --registry https://private.registry.com --api-key $API_KEY
```

## Anwendungsupdates

### Auf Updates prüfen

```bash
# Auf App-Updates prüfen
boxlang update check myapp

# Alle App-Updates prüfen
boxlang update check --all
```

### Automatisches Update

```toml
# box.toml
[appbox.update]
enabled = true
channel = "stable"  # stable, beta, alpha
auto-check = true
check-interval = "1d"
```

### Manuelles Update

```bash
# App auf neueste Version aktualisieren
boxlang update myapp

# Auf bestimmte Version aktualisieren
boxlang update myapp --version 2.0.0

# Alle Apps aktualisieren
boxlang update --all
```

## Erweiterte Funktionen

### Delta-Updates

```bash
# Delta-Paket generieren
boxlang package --delta-from 1.0.0

# Delta-Update anwenden
boxlang update myapp --delta ./myapp-1.0.0-to-1.1.0.delta
```

### Anwendungsplugins

```toml
# box.toml
[appbox.plugins]
enabled = true
plugin-dir = "plugins"
```

### Sandbox-Konfiguration

```toml
# box.toml
[appbox.sandbox]
enabled = true
filesystem = "restricted"
network = "allowed"
permissions = ["camera", "microphone"]
```

## Vollständiges Beispiel

### Beispiel-Projektkonfiguration

```toml
# box.toml
[package]
name = "weather-app"
version = "1.2.0"
edition = "2024"

[appbox]
name = "Weather"
description = "Eine einfache Wetteranwendung"
icon = "assets/weather-icon.png"
category = "Utilities"

[appbox.permissions]
network = true
location = true
notifications = true

[appbox.resources]
assets = ["assets/**/*"]
themes = ["themes/**/*"]
locales = ["locales/**/*.json"]
```

### Vollständiger Verpackungsworkflow

```bash
# 1. Tests ausführen
boxlang test

# 2. Release-Version bauen
boxlang build --release

# 3. Anwendung verpacken
boxlang package --sign

# 4. Paket überprüfen
boxlang verify target/package/weather-app-1.2.0.appbox

# 5. Im Store veröffentlichen
boxlang publish target/package/weather-app-1.2.0.appbox
```

## Fehlerbehebung

### Verpackung fehlgeschlagen

```bash
# Detaillierte Logs anzeigen
boxlang package --verbose

# Cache bereinigen und erneut versuchen
boxlang clean
boxlang package
```

### Signierungsprobleme

```bash
# Schlüssel überprüfen
boxlang keygen --verify ./keys/release.pem

# Schlüssel neu generieren
boxlang keygen --force
```

### Veröffentlichung fehlgeschlagen

```bash
# Netzwerkverbindung prüfen
boxlang doctor --network

# Anmeldestatus überprüfen
boxlang login --status
```

## Nächste Schritte

- [BoxLang Beispielprojekte](https://github.com/box-ecosystem/examples) ansehen
- [zetboxos Anwendungsentwicklungsleitfaden](../../readme/zetboxos/README_DE.md) lesen
- [BoxLang Community](https://community.boxlang.dev) beitreten
