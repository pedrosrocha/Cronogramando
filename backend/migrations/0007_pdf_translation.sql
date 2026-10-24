CREATE TABLE pdf_extractions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    resource_url TEXT NOT NULL UNIQUE,
    connector TEXT NOT NULL,
    engine TEXT NOT NULL CHECK (engine IN ('normal', 'academic')),
    page_count INT NOT NULL DEFAULT 0,
    blocks JSONB NOT NULL,
    markdown TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX pdf_extractions_engine_idx ON pdf_extractions (engine);

CREATE TABLE pdf_translations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    resource_url TEXT NOT NULL,
    source_language TEXT NOT NULL,
    target_language TEXT NOT NULL,
    provider TEXT NOT NULL,
    blocks JSONB NOT NULL,
    markdown TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (resource_url, target_language)
);

CREATE INDEX pdf_translations_resource_url_idx ON pdf_translations (resource_url);
