CREATE TABLE IF NOT EXISTS organizations (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    email VARCHAR NOT NULL
);

CREATE TABLE IF NOT EXISTS employees (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    role VARCHAR NOT NULL,
    org_id INTEGER REFERENCES organizations(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS ingredients (
    id SERIAL PRIMARY KEY,
    lotcode VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    date DATE NOT NULL,
    org_id INTEGER REFERENCES organizations(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS recipes (
    id SERIAL PRIMARY KEY,
    lotcode VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    date_made DATE NOT NULL,
    org_id INTEGER REFERENCES organizations(id) ON DELETE CASCADE,
    description TEXT
);

CREATE TABLE IF NOT EXISTS recipe_ingredients (
    recipe_id INTEGER REFERENCES recipes(id) ON DELETE CASCADE,
    ingredient_id INTEGER REFERENCES ingredients(id) ON DELETE CASCADE,
    PRIMARY KEY (recipe_id, ingredient_id)
);