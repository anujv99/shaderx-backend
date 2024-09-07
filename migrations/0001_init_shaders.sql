CREATE TABLE IF NOT EXISTS shaders (
  id CHAR(6) PRIMARY KEY NOT NULL,
  name VARCHAR(255),
  description VARCHAR(8192),
  data JSONB,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW(),
  deleted BOOLEAN DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS shaders_name_idx ON shaders (name);
CREATE INDEX IF NOT EXISTS shaders_not_deleted_idx ON shaders (deleted) WHERE deleted = FALSE;

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
  NEW."updatedAt" = NOW();
  RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER set_updated_at
BEFORE UPDATE ON shaders
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();
