CREATE TABLE schedule_slots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id UUID NOT NULL REFERENCES steps(id) ON DELETE CASCADE,
    slot_date DATE NOT NULL,
    start_minute INT NOT NULL CHECK (start_minute >= 0 AND start_minute < 1440),
    end_minute INT NOT NULL CHECK (end_minute > start_minute AND end_minute <= 1440),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (step_id, slot_date)
);

CREATE INDEX schedule_slots_step_id_idx ON schedule_slots (step_id);
CREATE INDEX schedule_slots_date_idx ON schedule_slots (slot_date);
