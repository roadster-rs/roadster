CREATE TABLE IF NOT EXISTS "user" (
    "created_at" timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "id" uuid NOT NULL PRIMARY KEY,
    "name" varchar NOT NULL CHECK (CHAR_LENGTH("name") > 0),
    "username" varchar NOT NULL UNIQUE CHECK (CHAR_LENGTH("username") > 0),
    "email" varchar NOT NULL UNIQUE CHECK (CHAR_LENGTH("email") > 0),
    "password" varchar NOT NULL
)
