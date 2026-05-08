# Aufwendigen Map-Model-Cache vermeiden

Relation: File 1 -> * Note

```rs
File {
	referenced_notes: [NoteId],
}

enum ContextType {
	Text,
	File,
}

Note {
	context: String,
	context_type: ContextType,
}

type FilePath = string;
file_id_map: HashMap<FilePath, FileId>;
```

* File loeschen: ContextType von File -> Text
* Notiz loeschen: `FileId` raussuchen und wenn gefunden `NoteId` aus Modelle loeschen
* Notiz-Kontext File zu Text aendern: `FileId` raussuchen und wenn gefunden `NoteId` aus `referenced_notes`-Modell loeschen
* kann alles im Worker realisiert werden
