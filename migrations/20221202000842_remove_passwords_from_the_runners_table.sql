-- Add migration script here

ALTER TABLE "runners" DROP COLUMN "password", ALTER COLUMN "local_path" TYPE VARCHAR, ALTER COLUMN "local_path" SET NOT NULL;
