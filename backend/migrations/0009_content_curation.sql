-- Phase 04b: content curation and quality.
--
-- Activities can now carry a generated lesson body, a resolution trace, a
-- quality status and a human-readable selection reason. Feedback and probe
-- caches back the relevance loop.

ALTER TABLE activities ADD COLUMN content_body TEXT;
ALTER TABLE activities ADD COLUMN content_meta JSONB;
ALTER TABLE activities ADD COLUMN quality_status TEXT NOT NULL DEFAULT 'ok'
    CHECK (quality_status IN ('ok', 'fallback', 'rejected'));
ALTER TABLE activities ADD COLUMN selection_reason TEXT;

ALTER TABLE activities DROP CONSTRAINT activities_type_check;
ALTER TABLE activities ADD CONSTRAINT activities_type_check
    CHECK (type IN ('video', 'podcast', 'pdf', 'article', 'quiz', 'task', 'lesson'));

CREATE TABLE activity_feedback (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    activity_id UUID NOT NULL REFERENCES activities(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating SMALLINT NOT NULL CHECK (rating IN (-1, 1)),
    reasons TEXT[] NOT NULL DEFAULT '{}',
    comment TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (activity_id, user_id)
);

CREATE INDEX activity_feedback_user_id_idx ON activity_feedback (user_id);
CREATE INDEX activity_feedback_rating_idx ON activity_feedback (rating);

CREATE TABLE source_stats (
    connector TEXT NOT NULL,
    domain TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL,
    language TEXT NOT NULL,
    helpful INT NOT NULL DEFAULT 0,
    not_helpful INT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (connector, domain, category, language)
);

CREATE TABLE resource_probe_cache (
    url_hash TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    kind TEXT NOT NULL,
    page_count INT,
    word_count INT,
    text_chars INT,
    language TEXT,
    ok BOOLEAN NOT NULL,
    reason TEXT,
    expires_at TIMESTAMPTZ NOT NULL
);
