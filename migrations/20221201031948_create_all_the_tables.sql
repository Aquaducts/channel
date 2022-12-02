-- Add migration script here

CREATE TABLE IF NOT EXISTS "runners" (
    "name" VARCHAR PRIMARY KEY,
    "id" BIGSERIAL NOT NULL,
    "local_path" VARCHAR,
    -- A password is required because you can obviously fuck some shit up without one.
    "password" VARCHAR NOT NULL,
    "created_at" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc')
);

CREATE TABLE IF NOT EXISTS "repos" (
    "id" BIGSERIAL PRIMARY KEY,
    "gh_id" BIGINT NOT NULL,
    "owner" VARCHAR NOT NULL,
    "name" VARCHAR NOT NULL
);

CREATE TABLE IF NOT EXISTS "job" (
    "id" VARCHAR PRIMARY KEY,
    "assigned_runner" VARCHAR REFERENCES "runners"("name"),
    -- Status 0 = Queued, 1 = Running, 2 = Failed, 3 = Successful, 4 = Unkown / Something weird happened ig
    "status" BIGINT NOT NULL DEFAULT 0,
    "repo" BIGINT NOT NULL REFERENCES "repos"("id")
    -- TODO: created triggered_by once i get hecking users created.
)