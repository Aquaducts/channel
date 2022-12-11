-- Add migration script here

CREATE TABLE IF NOT EXISTS "job_logs" (
    "id" BIGSERIAL PRIMARY KEY,
    "job" BIGINT NOT NULL REFERENCES "job"("id"),
    "step" VARCHAR NOT NULL,
    -- 0 = success
    -- 1 = error
    -- 2 = unkown
    "status" BIGINT NOT NULL DEFAULT 0,
    "output" VARCHAR NOT NULL
);