CREATE TABLE todos
  ( id   INTEGER PRIMARY KEY AUTOINCREMENT
  , name VARCHAR(256) NOT NULL UNIQUE
  , done INTEGER NOT NULL DEFAULT 0
  );

CREATE TABLE deps
  ( id      INTEGER PRIMARY KEY AUTOINCREMENT
  , id_from INTEGER NOT NULL
  , id_to   INTEGER NOT NULL
  , FOREIGN KEY(id_from) REFERENCES todos(id)
  , FOREIGN KEY(id_to)   REFERENCES todos(id)
  );
