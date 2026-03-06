# 🦀 Woodpecker CI Setup für Rust

Dieses Setup optimiert deine CI-Pipeline durch ein benutzerdefiniertes Docker-Image. Dadurch entfällt das minütliche Installieren von System-Abhängigkeiten bei jedem Build.

## 1. Das Docker-Image (`Dockerfile.ci`)

Dieses Image enthält alle für Rust und Linux-Grafikbibliotheken notwendigen Abhängigkeiten. Speichere dies im Hauptverzeichnis deines Projekts.

```dockerfile
# Nutze das offizielle Rust-Image als Basis
FROM rust:1.80-slim

# Installiere System-Abhängigkeiten für Grafik und Input
RUN apt-get update && apt-get install -y \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libxkbcommon-dev \
    libxkbcommon-x11-dev \
    libudev-dev \
    libinput-dev \
    libfontconfig-dev \
    pkg-config \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Arbeitsverzeichnis setzen
WORKDIR /app

```

---

## 2. Die Woodpecker-Konfiguration

Erstelle den Ordner `.woodpecker/` und lege dort die folgenden zwei Dateien ab.

### Datei: `.woodpecker/01-image.yml`

Baut das Image neu, sobald du das `Dockerfile.ci` anpasst.

```yaml
kind: pipeline
name: build-docker-image

when:
  event: push
  path: "Dockerfile.ci"

steps:
  publish:
    image: woodpecker/plugin-docker-buildx
    settings:
      dockerfile: Dockerfile.ci
      repo: dein-nutzer/rust-ci-image
      tags: latest
      username:
        from_secret: docker_username
      password:
        from_secret: docker_password

```

### Datei: `.woodpecker/02-build.yml`

Führt die eigentlichen Tests aus. Nutzt dein fertiges Image für maximale Geschwindigkeit.

```yaml
kind: pipeline
name: rust-test-and-build

# Wartet auf die Image-Pipeline, falls diese läuft
depends_on:
  - build-docker-image

steps:
  test:
    image: dein-nutzer/rust-ci-image:latest
    environment:
      - CARGO_TERM_COLOR=always
    commands:
      - cargo build --verbose
      - cargo test --verbose

```

---

## 3. Vorbereitungen im Woodpecker UI

Bevor du den ersten `git push` machst, musst du folgende **Secrets** in den Repository-Einstellungen deines Woodpecker-Dashboards hinzufügen:

* **`docker_username`**: Dein Nutzername für Docker Hub (oder deine Registry).
* **`docker_password`**: Dein Passwort oder Access-Token.

---

### Was du jetzt gewonnen hast:

1. **Speed**: Dein Test-Step startet in Sekunden statt Minuten.
2. **Ordnung**: Deine Test-Pipeline ist extrem sauber und fokussiert sich nur auf `cargo`.
3. **Zuverlässigkeit**: Abhängigkeiten sind im Image "eingefroren" und ändern sich nicht unerwartet.
