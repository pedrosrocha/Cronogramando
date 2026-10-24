ALTER TABLE books
    ADD COLUMN completed_at TIMESTAMPTZ,
    ADD COLUMN reconciliation_status TEXT
        CHECK (reconciliation_status IN (
            'on_time', 'pending', 'stay_on_deadline', 'push_deadline', 'forced_push'
        )),
    ADD COLUMN reconciliation_warning TEXT;

CREATE INDEX books_reconciliation_status_idx ON books (reconciliation_status);
