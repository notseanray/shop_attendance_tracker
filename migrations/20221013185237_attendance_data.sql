-- Add migration script here
CREATE TABLE IF NOT EXISTS attendance_data (
    ID              TEXT NOT NULL,
    first_name      TEXT NOT NULL,
    last_name       TEXT NOT NULL,
    badge           BOOLEAN NOT NULL DEFAULT FALSE,
    grad_year       UNSIGNED INTEGER NOT NULL DEFAULT 0,
    creation_date   TEXT NOT NULL
);
