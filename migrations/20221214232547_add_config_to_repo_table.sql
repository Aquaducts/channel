-- Add migration script here

CREATE TABLE IF NOT EXISTS "spurs" (
    "id" BIGSERIAL PRIMARY KEY,
    "name" VARCHAR NOT NULL,
    "owned_by" BIGINT NOT NULL REFERENCES "repos"("id"),
    "steps" JSON[] NOT NULL
);