-- Create the receiving log table for tracking ingredient receipts
CREATE TABLE IF NOT EXISTS receiving_log (
    id SERIAL PRIMARY KEY,
    lotcode VARCHAR(255) NOT NULL,
    company_name VARCHAR(255) NOT NULL,
    item_name VARCHAR(255) NOT NULL,
    temperature VARCHAR(50) NOT NULL,
    date DATE NOT NULL,
    org_id INTEGER REFERENCES organizations(id) ON DELETE CASCADE
);