-- Add migration script here

ALTER TABLE "job" DROP COLUMN "id", ADD COLUMN "id" BIGSERIAL, ADD PRIMARY KEY ("id");