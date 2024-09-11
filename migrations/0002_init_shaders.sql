CREATE TABLE IF NOT EXISTS shaders (
  id CHAR(6) PRIMARY KEY NOT NULL,
  user_id INT NOT NULL,
  name VARCHAR(255),
  description VARCHAR(8192),
  data JSONB,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW(),
  deleted BOOLEAN DEFAULT FALSE,
  public BOOLEAN DEFAULT FALSE,
  FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS shaders_name_idx ON shaders (name);
CREATE INDEX IF NOT EXISTS shaders_not_deleted_idx ON shaders (deleted) WHERE deleted = FALSE;

CREATE TRIGGER set_updated_at
BEFORE UPDATE ON shaders
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();
