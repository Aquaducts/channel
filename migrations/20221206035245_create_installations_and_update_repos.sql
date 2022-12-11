-- Add migration script here

CREATE TABLE IF NOT EXISTS "installations" (
    "id" BIGINT PRIMARY KEY
);

ALTER TABLE "repos" ADD COLUMN "install" BIGINT REFERENCES "installations"("id");