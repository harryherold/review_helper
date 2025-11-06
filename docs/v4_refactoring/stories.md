# Hinzufuegen von Repository

* Jeweils ueber das Klicken des Add-Button initieren
* Oeffnen des File-Dialoags und auswahl des Repositories
* Wird abgebrochen, wird die Eingabe verworfen
* Wird bestaetigt geht es weiter ueber [Speichern im Filesystem](#speichern-im-filesystem)

## Speichern im Filesystem

### Keine existierende Repositories

* Pruefen `Repo-Pfad` ob es Git enthaelt
    * Wenn nicht Fehler an UI und Abbruch
* Anlegen des ReviewHelper-Verzeichnis im `AppData`
* [Repository erstellen](#repository-erstellen)

### Vorhandene Repositories

* Pruefen ob `Repo-Name` und `Repo-Pfad` schon existiert
    * Wenn nicht Fehler an UI und Abbruch
* Pruefen `Repo-Pfad` ob es Git enthaelt
    * Wenn nicht Fehler an UI und Abbruch
* [Repository erstellen](#repository-erstellen)

### Repository erstellen

* Anlegen des Repo-Verzeichnis mit `Repo-Name`
* Erstellen einer toml mit dem Namen `repository.toml`
    * Name: `Repo-Name`
    * First-Commit: muss ermittelt werden
    * Path: `Repo-Pfad`