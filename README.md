# RUST Actix + SQLx API
This Rust application is a web server that provides a RESTful API for managing organizations, employees, recipes, and ingredients in what appears to be a food production or recipe management system. Here's a breakdown of its functionality:

## Main Components:

Uses Actix Web framework for handling HTTP requests
PostgreSQL database for data storage
SQLX for database interactions
Serde for JSON serialization/deserialization


### Data Models:

Organization: Represents a company with ID, name, and email
Employee: Represents staff with ID, name, role, and organization ID
Recipe: Contains ID, lot code, name, creation date, organization ID, ingredients list, and description
Ingredient: Has ID, lot code, name, date, and organization ID


### Database Structure:

Five tables: organizations, employees, ingredients, recipes, and recipe_ingredients
The recipe_ingredients table creates a many-to-many relationship between recipes and ingredients
Foreign key relationships ensure data integrity


### API Endpoints:

Health check: /health returns server status
Organization endpoints:

Create: POST to /api/orgs
Get all: GET to /api/orgs
Get one: GET to /api/orgs/{id}
Update: PUT to /api/orgs/{id}
Delete: DELETE to /api/orgs/{id}



Note: Employee, recipe, and ingredient endpoints are mentioned in comments but not fully implemented in this code.

## To run

Make sure you have **Docker** installed 

Acquire base image

    docker pull postgres:17.4-alpine3.21
    docker pull rust:1.85.0-bookworm 

This may require

    docker login


Navigate to base directory

    docker-compose up -build

If this fails and you are running it with `sudo` don't. 
Instead:

    `sudo chown -R $(whoami) ~/.docker`

