-- Phase 02b: objective-driven decomposition and planning.
--
-- Every level of the plan now carries a measurable objective:
--   goal  -> outcome + success criteria
--   book  -> module objective + scope + prerequisites + assessment
--   step  -> learning outcome + success criteria + content intent
--
-- The defaults keep existing rows and the mock planners working until the
-- new prompts land.

ALTER TABLE goals ADD COLUMN outcome TEXT NOT NULL DEFAULT '';
ALTER TABLE goals ADD COLUMN success_criteria JSONB NOT NULL DEFAULT '[]'::jsonb;

ALTER TABLE books ADD COLUMN objective TEXT NOT NULL DEFAULT '';
ALTER TABLE books ADD COLUMN scope_in JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE books ADD COLUMN scope_out JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE books ADD COLUMN prerequisites JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE books ADD COLUMN assessment TEXT NOT NULL DEFAULT '';

ALTER TABLE steps ADD COLUMN learning_outcome TEXT NOT NULL DEFAULT '';
ALTER TABLE steps ADD COLUMN success_criteria TEXT NOT NULL DEFAULT '';
ALTER TABLE steps ADD COLUMN content_intent TEXT NOT NULL DEFAULT 'explanation'
    CHECK (content_intent IN ('explanation', 'worked_example', 'practice', 'assessment', 'review'));
