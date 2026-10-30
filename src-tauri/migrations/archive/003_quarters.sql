-- 003_quarters.sql
-- A quarter is a three-month context belonging to a year theme.
CREATE TABLE quarters (
    id TEXT PRIMARY KEY,
    year_theme_id TEXT NOT NULL REFERENCES year_themes(id) ON DELETE CASCADE,
    code TEXT NOT NULL CHECK(code IN ('Q1','Q2','Q3','Q4')),
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'upcoming',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(year_theme_id, code)
);