-- Legacy en-US values cannot be distinguished from the former implicit
-- default, so treat them as system. Preserve only explicit supported locales.
UPDATE workspaces
SET locale = CASE
    WHEN locale IN ('es-ES', 'de-DE', 'fr-FR') THEN locale
    ELSE 'system'
END;
