use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use postgres::{ Client, NoTls };
use postgres::Error as PostgresError;
use std::net::{ TcpListener, TcpStream };
use std::io::{ Read, Write };
use std::env;

#[macro_use]
extern crate serde_derive;

// Organization entity (renamed from User)
#[derive(Serialize, Deserialize)]
struct Organization {
    id: Option<i32>,
    name: String,
    email: String,
}

#[derive(Serialize, Deserialize)]
struct Employee {
    id: Option<i32>,
    name: String,
    role: String,
    org_id: i32,
}

#[derive(Serialize, Deserialize)]
struct Recipe {
    id: Option<i32>,
    lotcode: String,
    name: String,
    date_made: String,  // Consider using chrono::NaiveDate
    org_id: i32,
    ingredients: Vec<i32>,  // List of ingredient IDs
    description: String,
}

#[derive(Serialize, Deserialize)]
struct Ingredient {
    id: Option<i32>,
    lotcode: String,
    name: String,
    date: String,  // Consider using chrono::NaiveDate
    org_id: i32,
}

// Application state
struct AppState {
    db_pool: Pool<Postgres>,
}

// Health check endpoint
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "healthy"}))
}

// Organization endpoints
async fn create_organization(
    org: web::Json<Organization>,
    data: web::Data<AppState>,
) -> impl Responder {
    match sqlx::query_unchecked!(
        "INSERT INTO organizations (name, email) VALUES ($1, $2) RETURNING id",
        org.name,
        org.email
    )
    .fetch_one(&data.db_pool)
    .await
    {
        Ok(record) => {
            let created_org = Organization {
                id: Some(record.id),
                name: org.name.clone(),
                email: org.email.clone(),
            };
            HttpResponse::Created().json(created_org)
        }
        Err(e) => {
            eprintln!("Failed to create organization: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create organization"}))
        }
    }
}

async fn get_organization(
    path: web::Path<i32>,
    data: web::Data<AppState>,
) -> impl Responder {
    let id = path.into_inner();
    
    match sqlx::query_as_unchecked!(
        Organization,
        "SELECT id, name, email FROM organizations WHERE id = $1",
        id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(org)) => HttpResponse::Ok().json(org),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Organization not found"})),
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

async fn get_all_organizations(
    data: web::Data<AppState>,
) -> impl Responder {
    //TODO: REMOVE QUERY UNCHECKED SO THAT THE QUERYS ARENOT CHECKED AT RUNTIME
    match sqlx::query_as_unchecked!(
        Organization,
        "SELECT id, name, email FROM organizations"
    )
    .fetch_all(&data.db_pool)
    .await
    {
        Ok(orgs) => HttpResponse::Ok().json(orgs),
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

async fn update_organization(
    path: web::Path<i32>,
    org: web::Json<Organization>,
    data: web::Data<AppState>,
) -> impl Responder {
    let id = path.into_inner();
    
    match sqlx::query_unchecked!(
        "UPDATE organizations SET name = $1, email = $2 WHERE id = $3 RETURNING id",
        org.name,
        org.email,
        id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(_)) => {
            let updated_org = Organization {
                id: Some(id),
                name: org.name.clone(),
                email: org.email.clone(),
            };
            HttpResponse::Ok().json(updated_org)
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Organization not found"})),
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

async fn delete_organization(
    path: web::Path<i32>,
    data: web::Data<AppState>,
) -> impl Responder {
    let id = path.into_inner();
    
    match sqlx::query_unchecked!("DELETE FROM organizations WHERE id = $1 RETURNING id", id)
        .fetch_optional(&data.db_pool)
        .await
    {
        Ok(Some(_)) => HttpResponse::NoContent().finish(),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Organization not found"})),
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

// Initialize database tables
async fn init_database(pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    sqlx::query_unchecked!(
        "CREATE TABLE IF NOT EXISTS organizations (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            email VARCHAR NOT NULL
        )"
    )
    .execute(pool)
    .await?;

    sqlx::query_unchecked!(
        "CREATE TABLE IF NOT EXISTS employees (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            role VARCHAR NOT NULL,
            org_id INTEGER REFERENCES organizations(id) ON DELETE CASCADE
        )"
    )
    .execute(pool)
    .await?;

    sqlx::query_unchecked!(
        "CREATE TABLE IF NOT EXISTS ingredients (
            id SERIAL PRIMARY KEY,
            lotcode VARCHAR NOT NULL,
            name VARCHAR NOT NULL,
            date DATE NOT NULL,
            org_id INTEGER REFERENCES organizations(id) ON DELETE CASCADE
        )"
    )
    .execute(pool)
    .await?;

    sqlx::query_unchecked!(
        "CREATE TABLE IF NOT EXISTS recipes (
            id SERIAL PRIMARY KEY,
            lotcode VARCHAR NOT NULL,
            name VARCHAR NOT NULL,
            date_made DATE NOT NULL,
            org_id INTEGER REFERENCES organizations(id) ON DELETE CASCADE,
            description TEXT
        )"
    )
    .execute(pool)
    .await?;

    sqlx::query_unchecked!(
        "CREATE TABLE IF NOT EXISTS recipe_ingredients (
            recipe_id INTEGER REFERENCES recipes(id) ON DELETE CASCADE,
            ingredient_id INTEGER REFERENCES ingredients(id) ON DELETE CASCADE,
            PRIMARY KEY (recipe_id, ingredient_id)
        )"
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();
    
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    // Set up database connection pool
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@db:5432/postgres".to_string());

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create database connection pool");
    
    // Initialize database tables
    if let Err(e) = init_database(&db_pool).await {
        eprintln!("Error setting up database: {}", e);
        std::process::exit(1);
    }
    
    log::info!("Starting server at http://0.0.0.0:8080");
    
    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                db_pool: db_pool.clone(),
            }))
            .route("/health", web::get().to(health_check))
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/orgs")
                            .route("", web::post().to(create_organization))
                            .route("", web::get().to(get_all_organizations))
                            .route("/{id}", web::get().to(get_organization))
                            .route("/{id}", web::put().to(update_organization))
                            .route("/{id}", web::delete().to(delete_organization))
                    )
                    // Add additional routes for employees, recipes, and ingredients here
            )
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}