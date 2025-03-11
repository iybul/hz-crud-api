# API Testing Guide

This directory contains integration tests for the CRUD API endpoints.

## Running Tests

To run all tests:
```
cargo test
```

To run tests with output:
```
cargo test -- --nocapture
```

To run a specific test:
```
cargo test test_create_organization
```

## Manual Testing with curl

You can also test the API endpoints manually using curl:

### Health Check
```
curl http://localhost:8080/health
```

### Organizations
```
# Create organization
curl -X POST http://localhost:8080/api/orgs \
  -H "Content-Type: application/json" \
  -d '{"name":"Test Org","email":"test@example.com"}'

# Get all organizations
curl http://localhost:8080/api/orgs

# Get organization by ID
curl http://localhost:8080/api/orgs/1

# Update organization
curl -X PUT http://localhost:8080/api/orgs/1 \
  -H "Content-Type: application/json" \
  -d '{"name":"Updated Org","email":"updated@example.com"}'

# Delete organization
curl -X DELETE http://localhost:8080/api/orgs/1
```

## Test Database Setup

The tests use the database configuration from your .env file. Make sure your database is running:

```
docker-compose up -d db
```

## Adding New Tests

To add a new test:
1. Add a new test function in api_test.rs using the `#[actix_rt::test]` attribute
2. Follow the existing pattern for setting up the test app and making requests
3. Use unique test data with UUIDs to avoid conflicts between tests