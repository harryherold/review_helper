# Diff/Re-Diff

Hier ist eine detaillierte Analyse und Diskussion der drei Handlungsoptionen (A, B, C) zur Erneuerung der geladenen File-Diffs (`loaded_file_diffs`), wenn ein Nutzer einen **Re-Diff** auslöst.

---

## 1. Problemstellung & Kontext

Beim Ausführen eines Re-Diffs (z. B. wenn sich Commit-SHAs ändern, neue Commits hinzukommen oder gegen eine geänderte Working Copy gedifft wird) verändern sich zwei Ebenen:
1. **Dateiebene (`file_diff_model`)**: Liste der geänderten Dateien, Hinzufügungen/Löschungen, Change-Types.
2. **Zeilenebene (`loaded_file_diffs`)**: Die im Speicher/UI gepufferten Zeilen (`SlintDiffLines`), Minimaps und Zeilenstatus der aktuell oder zuvor geöffneten Dateien.

Wird `loaded_file_diffs` beim Re-Diff **nicht** aktualisiert oder geleert, kommt es zu Inkonsistenzen:
- `create_or_get_diff_model_entry` sieht, dass eine `file_diff_id` bereits in `loaded_file_diffs` vorhanden ist, und fordert die neuen Zeilen vom Worker **nicht** neu an.
- Die UI würde veraltete Zeilen, falsche Zeilennummern oder Zeilen von nicht mehr existierenden Diffs anzeigen.

---

## 2. Detaillierte Bewertung der Reaktionen (A, B, C)

### Option A: `loaded_file_diffs` prüfen und ggf. erneuern (Vollständiger Sync)

* **Ablauf**: 
  Alle in `loaded_file_diffs` vorhandenen File-Diffs werden gegen die neuen Diff-Ergebnisse geprüft.
  - Wenn eine Datei im neuen Diff nicht mehr existiert: Eintrag entfernen.
  - Wenn eine Datei weiterhin existiert: `WorkerMessage::LoadFileLineDifferences` für jedes geladene File-Diff auslösen, um den Inhalt in-place zu aktualisieren.

* **Vorteile**:
  * **Nahtlose UX**: Wenn der Nutzer mehrere Dateien geöffnet/expandiert hatte, bleiben alle Ansichten geladen und werden im Hintergrund aktualisiert.

* **Nachteile / Herausforderungen**:
  * **Höhere Komplexität & Performance-Load**: Bei vielen bereits geladenen Dateien müssen mehrere Git-Diffs gleichzeitig neu berechnet, geparst und über den Worker an die UI gesendet werden.
  * **State-Inkonsistenzen**: Bei verschobenen oder stark veränderten Zeilen können Scroll-Positionen, Minimaps und Zeilennotizen springen oder ungültig werden.

---

### Option B: `loaded_file_diffs` komplett löschen und Anzeige zurücksetzen (Full Reset)

* **Ablauf**: 
  `loaded_file_diffs.clear()` ausführen und die Auswahl der aktiven Datei zurücksetzen (`selected_id = -1`). Die File-Diff-Detailansicht wird geleert.

* **Vorteile**:
  * **Sehr einfache Implementierung**: Absolut sauberer Zustand, null Risiko von Cache-Inkonsistenzen oder Speicherlecks.
  * **Maximale Performance**: Keine unnötigen Git-Diffs für ungenutzte Dateien.

* **Nachteile**:
  * **Disruptive UX**: Der Nutzer verliert sofort den Fokus auf die Datei, die er gerade betrachtet hat. Er muss nach dem Re-Diff die Datei in der Liste suchen und erneut anklicken.

---

### Option C: `loaded_file_diffs` löschen bis auf den aktiven File-Diff (Empfohlene Lösung)

* **Ablauf**:
  1. Bestimme die aktuell ausgewählte/sichtbare `active_file_diff_id`.
  2. Entferne alle **inaktiven** File-Diffs aus `loaded_file_diffs`.
  3. Prüfe, ob die `active_file_diff_id` im neuen `file_diff_model` noch existiert:
     * **JA (Datei hat weiterhin Änderungen)**: Leere die Diff-Zeilen für diese ID und löse ein neues `LoadFileLineDifferences` im Worker aus. Der Diff der aktiven Datei wird direkt neu geladen und flüssig aktualisiert.
     * **NEIN (Datei ist unverändert, gelöscht oder gemergt)**: Entferne auch diesen Eintrag aus `loaded_file_diffs` und setze die aktive Auswahl auf `-1` (File-Diff-Anzeige wird leer).

* **Vorteile**:
  * **Beste User Experience (UX)**: Der Nutzer behält genau den Fokus auf der Datei, an der er gerade arbeitet. Er sieht nach dem Klick auf "Re-Diff" sofort das aktualisierte Resultat seiner Datei.
  * **Geringe Komplexität**: Es muss höchstens **eine** Datei neu geladen werden.
  * **Ressourcenschonend**: Früher geladene, inaktive Diffs geben ihren Speicherplatz sofort frei.

* **Nachteile**:
  * Minimaler Zusatzaufwand gegenüber Option B (Überprüfung, ob die aktive Datei im neuen Diff noch vorhanden ist).

---

## 3. Empfehlung & Umsetzungsvorschlag

**Option C** bietet das optimale Verhältnis aus hervorragender UX, geringer Komplexität und hoher Performance.

### Technischer Ablauf bei Option C:

1. **Im Worker (`find_file_difference` in `worker_impl.rs`)**:
   Nachdem das neue `file_diff_map` berechnet und `review.file_diffs` aktualisiert wurden:
   ```rust
   // Bestimme, welche Datei-IDs im neuen Diff existieren
   let valid_ids: HashSet<usize> = review.file_diffs.keys().map(|id| id.as_usize()).collect();

   // Benachrichtige den UiUpdater
   self.ui_updater.sync_loaded_file_diffs(repository_id.as_usize(), review_id.as_usize(), valid_ids, active_file_diff_id);
   ```

2. **Im `UiUpdater` (`ui_updater.rs`)**:
   ```rust
   pub fn sync_loaded_file_diffs(&self, repository_id: usize, review_id: usize, valid_ids: HashSet<usize>, active_id: Option<usize>) {
       self.execute_in_event_loop(move |app_window| {
           let review = model_utils::get_slint_review(&app_window, repository_id, review_id)...;
           let loaded_diffs = cast_model!(review.loaded_file_diffs, IdModel<ui::SlintDiffLines>);

           // 1. Inaktive / ungültige Diffs entfernen
           loaded_diffs.clear(); 

           // 2. Aktive Datei behandeln, falls sie im neuen Diff existiert
           if let Some(active_id) = active_id {
               if valid_ids.contains(&active_id) {
                   // Leeres Modell anlegen und Zeilen neu anfordern
                   let lines = Rc::new(VecModel::default());
                   let diff_lines = ui::SlintDiffLines { ... };
                   loaded_diffs.add(active_id, diff_lines);

                   // Worker auffordern, die neuen Diff-Zeilen für die aktive Datei zu laden
                   channel.send(WorkerMessage::LoadFileLineDifferences { ... });
               } else {
                   // Aktive Datei existiert im neuen Diff nicht mehr -> Ansicht zurücksetzen
                   app_window.global::<ui::SlintFileDiffCallbacks>().reset_active_selection();
               }
           }
       });
   }
   ```

### Fazit
Option C verhindert veraltete Diffs im Cache, hält den Speicherbedarf gering und bietet dem Anwender ein direktes, unterbrechungsfreies Feedback in der aktuell geöffneten Datei.
