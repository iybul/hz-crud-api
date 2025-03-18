use actix_web::{web, App, HttpResponse, HttpServer, Responder, middleware::Logger, HttpRequest, Error as ActixWebError};
use actix_cors::Cors;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, types::chrono::NaiveDate};
use std::env;
use chrono::NaiveDate;

// Import auth module
mod auth;
use auth::{verify_auth, get_token_from_request};

// Organization entity
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Organization {
    pub id: Option<i32>,
    pub name: String,
    pub email: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct BatchInput {
    pub org_id: i32,
    pub employee: String,
    pub recipe_lotcode: String,
    #[serde(alias = "batchLotCode")]
    pub batch_lot_code: String,
    pub ingredients: Vec<i32>,
    pub amount_ingredients: Vec<i32>,
    pub date_made: String,
    pub amount_made: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Batch {
    pub id: Option<i32>,
    pub org_id: i32,
    pub employee: String,
    pub recipe_lotcode: String,
    #[serde(alias = "batchLotCode")]
    pub batch_lot_code: String,
    pub ingredients: Vec<i32>,
    pub amount_ingredients: Vec<i32>,
    pub date_made: String,
    pub amount_made: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Employee {
    pub id: Option<i32>,
    pub name: String,
    pub role: String,
    pub org_id: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct RecipeInput {
    pub lotcode: String,
    pub name: String,
    pub date_made: String, // For user input as string
    pub org_id: i32,
    pub ingredients: Vec<i32>,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Recipe {
    id: Option<i32>,
    lotcode: String,
    name: String,
    date_made: String,  
    org_id: i32,
    ingredients: Vec<i32>,  // List of ingredient IDs
    description: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct IngredientInput {
    pub lotcode: String,
    pub name: String,
    pub date: String, // For user input as string
    pub org_id: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Ingredient {
    id: Option<i32>,
    lotcode: String,
    name: String,
    date: String,  
    org_id: i32,
}

// Application state
pub struct AppState {
    db_pool: Pool<Postgres>,
}

// Helper function to extract organization ID from token
async fn get_org_id_from_token(
    req: &HttpRequest, 
    db_pool: &Pool<Postgres>
) -> Result<i32, auth::AuthError> {
    if let Some(token) = get_token_from_request(req) {
        let token_record = sqlx::query!(
            r#"
            SELECT org_id, expires_at, is_revoked
            FROM access_tokens
            WHERE token = $1
            "#,
            token
        )
        .fetch_optional(db_pool)
        .await
        .map_err(auth::AuthError::DatabaseError)?;
        
        match token_record {
            Some(record) => {
                if record.is_revoked {
                    return Err(auth::AuthError::Unauthorized("Token has been revoked".to_string()));
                }
                
                let expires_at = record.expires_at;
                let now = chrono::Utc::now().naive_utc();
                
                if expires_at < now {
                    return Err(auth::AuthError::Unauthorized("Token has expired".to_string()));
                }
                
                Ok(record.org_id)
            }
            None => Err(auth::AuthError::Unauthorized("Invalid token".to_string())),
        }
    } else {
        Err(auth::AuthError::Unauthorized("No authorization token provided".to_string()))
    }
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
    match sqlx::query!(
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
    req: HttpRequest,
) -> impl Responder {
    // Get authenticated organization ID
    let auth_org_id = match get_org_id_from_token(&req, &data.db_pool).await {
        Ok(id) => id,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };
    
    let id = path.into_inner();
    
    // Only allow access to the authenticated organization's data
    if id != auth_org_id {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "Access denied"}));
    }
    
    match sqlx::query_as!(
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
    req: HttpRequest,
) -> impl Responder {
    // Get authenticated organization ID
    let auth_org_id = match get_org_id_from_token(&req, &data.db_pool).await {
        Ok(id) => id,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };
    
    // Only return the authenticated organization
    match sqlx::query_as!(
        Organization,
        "SELECT id, name, email FROM organizations WHERE id = $1",
        auth_org_id
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
    req: HttpRequest,
) -> impl Responder {
    // Get authenticated organization ID
    let auth_org_id = match get_org_id_from_token(&req, &data.db_pool).await {
        Ok(id) => id,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };
    
    let id = path.into_inner();
    
    // Only allow updating the authenticated organization
    if id != auth_org_id {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "Access denied"}));
    }
    
    match sqlx::query!(
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
    req: HttpRequest,
) -> impl Responder {
    // Get authenticated organization ID
    let auth_org_id = match get_org_id_from_token(&req, &data.db_pool).await {
        Ok(id) => id,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };
    
    let id = path.into_inner();
    
    // Only allow deleting the authenticated organization
    if id != auth_org_id {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "Access denied"}));
    }
    
    match sqlx::query!("DELETE FROM organizations WHERE id = $1 RETURNING id", id)
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

// Configure app with database pool
pub fn configure_app(config: &mut web::ServiceConfig, db_pool: Pool<Postgres>) {
    config
        .app_data(web::Data::new(AppState {
            db_pool: db_pool.clone(),
        }))
        .route("/health", web::get().to(health_check))
        .service(
            web::scope("/api")
                // Authentication endpoints
                .service(
                    web::scope("/auth")
                        .route("/register", web::post().to(auth::register_organization))
                        .route("/login", web::post().to(auth::login_organization))
                        .route("/logout", web::post().to(auth::logout_organization))
                )
                // Organization endpoints
                .service(
                    web::scope("/orgs")
                        .route("", web::post().to(create_organization))
                        .route("", web::get().to(get_all_organizations))
                        .route("/{id}", web::get().to(get_organization))
                        .route("/{id}", web::put().to(update_organization))
                        .route("/{id}", web::delete().to(delete_organization))
                )
                // Add additional routes for employees, recipes, and ingredients here
        );
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();
    
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    // Set up database connection pool
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/postgres".to_string());
        
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create database connection pool");
    
    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run database migrations");
    
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let server_url = format!("{}:{}", host, port);
    
    println!("Starting server at http://{}", server_url);
    
    // Start HTTP server
    HttpServer::new(move || {
        // Configure CORS
        let cors = Cors::default()
            .allow_any_origin()  // In production, limit this to your frontend origin
            .allow_any_method()
            .allow_any_header()
            .supports_credentials()
            .max_age(3600);
        
        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .configure(|config| {
                configure_app(config, db_pool.clone());
            })
    })
    .bind(server_url)?
    .run()
    .await
}