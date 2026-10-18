CREATE TABLE steps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    book_id UUID NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    order_index INT NOT NULL,
    title TEXT NOT NULL,
    activity_kind TEXT NOT NULL CHECK (activity_kind IN ('resource', 'quiz', 'task')),
    estimated_duration_mins INT NOT NULL DEFAULT 0,
    references_step_id UUID REFERENCES steps(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (book_id, order_index)
);

CREATE INDEX steps_book_id_idx ON steps (book_id);
CREATE INDEX steps_references_step_id_idx ON steps (references_step_id);

CREATE TABLE activities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id UUID NOT NULL REFERENCES steps(id) ON DELETE CASCADE,
    type TEXT NOT NULL CHECK (type IN ('video', 'podcast', 'pdf', 'article', 'quiz', 'task')),
    resource_url TEXT,
    embed_id TEXT,
    source_connector TEXT,
    language TEXT NOT NULL DEFAULT 'pt-BR',
    completed BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX activities_step_id_idx ON activities (step_id);
