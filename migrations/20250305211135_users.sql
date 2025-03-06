-- Add migration script here
-- Your SQL goes here
CREATE TABLE IF NOT EXISTS users (
  username VARCHAR NOT NULL PRIMARY KEY,
  password VARCHAR NOT NULL
);
