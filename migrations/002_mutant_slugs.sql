PRAGMA foreign_keys = ON;

DROP INDEX IF EXISTS idx_mutants_unique;

CREATE TABLE IF NOT EXISTS mutant_slugs (
    mutant_id INTEGER NOT NULL REFERENCES mutants(id) ON DELETE CASCADE,
    slug TEXT NOT NULL,
    is_primary INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (mutant_id, slug)
);

CREATE INDEX IF NOT EXISTS idx_mutant_slugs_slug
ON mutant_slugs (slug);

INSERT INTO mutant_slugs (mutant_id, slug, is_primary)
SELECT id, mutation_slug, 1
FROM mutants;

CREATE TEMPORARY TABLE mutant_canonical_map AS
SELECT
    m.id AS id,
    (
        SELECT MIN(id)
        FROM mutants m2
        WHERE m2.target_id = m.target_id
          AND m2.byte_offset = m.byte_offset
          AND m2.old_text = m.old_text
          AND m2.new_text = m.new_text
    ) AS canonical_id
FROM mutants m;

INSERT OR IGNORE INTO mutant_slugs (mutant_id, slug, is_primary)
SELECT
    mcm.canonical_id,
    m.mutation_slug,
    0
FROM mutants m
JOIN mutant_canonical_map mcm ON mcm.id = m.id
WHERE mcm.id <> mcm.canonical_id;

UPDATE outcomes
SET mutant_id = (
    SELECT canonical_id
    FROM mutant_canonical_map mcm
    WHERE mcm.id = outcomes.mutant_id
)
WHERE mutant_id IN (
    SELECT id FROM mutant_canonical_map WHERE id <> canonical_id
)
AND NOT EXISTS (
    SELECT 1
    FROM outcomes o2
    WHERE o2.mutant_id = (
        SELECT canonical_id
        FROM mutant_canonical_map mcm2
        WHERE mcm2.id = outcomes.mutant_id
    )
);

DELETE FROM mutants
WHERE id IN (
    SELECT id
    FROM mutant_canonical_map
    WHERE id <> canonical_id
);

DROP TABLE mutant_canonical_map;

CREATE UNIQUE INDEX IF NOT EXISTS idx_mutants_unique
ON mutants (target_id, byte_offset, old_text, new_text);
