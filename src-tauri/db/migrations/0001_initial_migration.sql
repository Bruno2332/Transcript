
CREATE TABLE transcripts (
     id INTEGER PRIMARY KEY AUTOINCREMENT,
     name TEXT NOT NULL,
     date_of_service TEXT NOT NULL,
     transcription TEXT NOT NULL,
     created_at TEXT NOT NULL,
     updated_at TEXT NOT NULL
);