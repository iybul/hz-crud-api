CREATE TABLE IF NOT EXISTS batches (
    id SERIAL PRIMARY KEY,
    org_id INTEGER REFERENCES organizations(id) ON DELETE CASCADE,
    employee VARCHAR NOT NULL,
    recipe_lotcode VARCHAR NOT NULL,
    batch_lot_code VARCHAR NOT NULL,
    date_made DATE NOT NULL,
    amount_made VARCHAR NOT NULL
);

CREATE TABLE IF NOT EXISTS batch_ingredients (
    batch_id INTEGER REFERENCES batches(id) ON DELETE CASCADE,
    ingredient_id INTEGER REFERENCES ingredients(id) ON DELETE CASCADE,
    amount INTEGER NOT NULL,
    PRIMARY KEY (batch_id, ingredient_id)
);