-- Add migration script here
-- Your SQL goes here
CREATE TABLE IF NOT EXISTS pipelineconfig (
  key VARCHAR NOT NULL PRIMARY KEY,
  value JSON
);
