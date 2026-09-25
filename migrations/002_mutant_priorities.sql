-- Heuristic judgments are independent of mutable test outcomes. A pre- and a
-- post-campaign judgment can coexist for each saved mutant.
CREATE TABLE mutant_priorities (
    mutant_id INTEGER NOT NULL REFERENCES mutants(id) ON DELETE CASCADE,
    purpose TEXT NOT NULL CHECK (purpose IN ('pre', 'post')),
    input_hash TEXT NOT NULL,
    model TEXT NOT NULL,
    created_at TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    PRIMARY KEY (mutant_id, purpose)
);
