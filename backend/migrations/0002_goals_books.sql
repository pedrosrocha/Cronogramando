CREATE TABLE goals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    why_statement TEXT,
    deadline DATE NOT NULL,
    target_deadline DATE NOT NULL,
    availability JSONB NOT NULL DEFAULT '{"days": ["mon", "wed", "fri"], "minutes_per_day": 60}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX goals_user_id_idx ON goals (user_id);

CREATE TABLE books (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    goal_id UUID NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
    order_index INT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'locked'
        CHECK (status IN ('locked', 'unlocked', 'in_progress', 'completed')),
    planned_duration_mins INT NOT NULL DEFAULT 0,
    min_duration_mins INT NOT NULL DEFAULT 0,
    actual_duration_mins INT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (goal_id, order_index)
);

CREATE INDEX books_goal_id_idx ON books (goal_id);
