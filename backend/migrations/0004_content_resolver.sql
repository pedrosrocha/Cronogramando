ALTER TABLE activities ADD COLUMN source_category TEXT;

CREATE INDEX activities_source_category_idx ON activities (source_category);

CREATE TABLE connector_usage (
    connector TEXT NOT NULL,
    usage_date DATE NOT NULL DEFAULT CURRENT_DATE,
    units INT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (connector, usage_date)
);
