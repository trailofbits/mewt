-- Create targets table
CREATE TABLE IF NOT EXISTS targets (
    id INTEGER NOT NULL PRIMARY KEY,
    path TEXT NOT NULL,
    file_hash TEXT NOT NULL UNIQUE,
    text TEXT NOT NULL,
    language TEXT NOT NULL
);

-- Create mutants table
CREATE TABLE IF NOT EXISTS mutants (
    id INTEGER NOT NULL PRIMARY KEY,
    target_id INTEGER NOT NULL,
    byte_offset INTEGER NOT NULL,
    line_offset INTEGER NOT NULL,
    old_text TEXT NOT NULL,
    new_text TEXT NOT NULL,
    mutation_slug TEXT NOT NULL DEFAULT 'unknown',
    FOREIGN KEY (target_id) REFERENCES targets(id) ON DELETE CASCADE
);

-- Ensure we can store multiple mutants on the same line so long as they are
-- distinct by position, text, and kind. This prevents accidental deduping of
-- different mutations that share a line.
CREATE UNIQUE INDEX IF NOT EXISTS idx_mutants_unique
ON mutants (target_id, byte_offset, old_text, new_text, mutation_slug);

-- Create outcomes table
CREATE TABLE IF NOT EXISTS outcomes (
    mutant_id INTEGER NOT NULL PRIMARY KEY,
    status TEXT NOT NULL,
    output TEXT NOT NULL,
    time TIMESTAMP NOT NULL,
    duration_ms INTEGER NOT NULL,
    FOREIGN KEY (mutant_id) REFERENCES mutants(id) ON DELETE CASCADE
); 