-- Create the problem_logs table
CREATE TABLE IF NOT EXISTS problem_logs (
    id SERIAL PRIMARY KEY,
    is_open BOOLEAN NOT NULL DEFAULT TRUE,
    date_opened DATE NOT NULL,
    customer_name VARCHAR(255) NOT NULL,
    problem_type VARCHAR(100) NOT NULL,
    problem_description TEXT NOT NULL,
    recall BOOLEAN NOT NULL DEFAULT FALSE,
    date_resolved DATE
);

-- Create the problem_logs_employees table for many-to-many relationship
CREATE TABLE IF NOT EXISTS problem_logs_employees (
    problem_log_id INTEGER REFERENCES problem_logs(id) ON DELETE CASCADE,
    employee_id INTEGER REFERENCES employees(id) ON DELETE CASCADE,
    PRIMARY KEY (problem_log_id, employee_id)
);