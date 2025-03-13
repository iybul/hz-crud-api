use actix_web::{web, HttpResponse, Responder, HttpRequest, Error as ActixWebError};
use actix_web::dev::ServiceRequest;
use actix_web::http::header;
use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use std::env;
use uuid::Uuid;

use crate::Organization;

// JWT Claims for token
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,     // Subject (org ID)
    pub exp: usize,      // Expiration time (as UTC timestamp)
    pub iat: usize,      // Issued at (as UTC timestamp)
    pub jti: String,     // JWT ID (unique identifier for this token)
}

// Login request payload
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

// Registration request payload
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

// Login response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub organization: Organization,
}

// Error types
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("Organization not found")]
    OrganizationNotFound,
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    
    #[error("JWT error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),
}

// Helper function to hash passwords
pub fn hash_password(password: &str) -> Result<(String, String), argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    let password_hash = argon2.hash_password(password.as_bytes(), &salt)?
        .to_string();
    
    Ok((password_hash, salt.to_string()))
}

// Helper function to verify passwords
pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(hash) => hash,
        Err(_) => return false,
    };
    
    Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
}

// Generate a JWT token
pub fn generate_token(org_id: i32) -> Result<String, AuthError> {
    let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "default_secret_change_me".to_string());
    let exp = Utc::now()
        .checked_add_signed(Duration::days(7))
        .expect("valid timestamp")
        .timestamp() as usize;
    
    let claims = Claims {
        sub: org_id.to_string(),
        exp,
        iat: Utc::now().timestamp() as usize,
        jti: Uuid::new_v4().to_string(),
    };
    
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(AuthError::JwtError)
}

// Verify a JWT token
pub fn verify_token(token: &str) -> Result<Claims, AuthError> {
    let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "default_secret_change_me".to_string());
    
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )?;
    
    Ok(token_data.claims)
}

// Extract token from request
pub fn get_token_from_request(req: &HttpRequest) -> Option<String> {
    req.headers().get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|auth_header| {
            if auth_header.starts_with("Bearer ") {
                Some(auth_header[7..].to_string())
            } else {
                None
            }
        })
}

// Registration handler
pub async fn register_organization(
    req: web::Json<RegisterRequest>,
    data: web::Data<crate::AppState>,
) -> impl Responder {
    // Hash the password
    let (password_hash, password_salt) = match hash_password(&req.password) {
        Ok((hash, salt)) => (hash, salt),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to hash password"
        })),
    };
    
    // Insert the new organization
    match sqlx::query!(
        r#"
        INSERT INTO organizations (name, email, password_hash, password_salt)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, email
        "#,
        req.name,
        req.email,
        password_hash,
        password_salt
    )
    .fetch_one(&data.db_pool)
    .await
    {
        Ok(record) => {
            let organization = Organization {
                id: Some(record.id),
                name: record.name,
                email: record.email,
            };
            
            // Generate JWT token
            match generate_token(record.id) {
                Ok(token) => {
                    // Store token in database
                    let expires_at = Utc::now()
                        .checked_add_signed(Duration::days(7))
                        .expect("valid timestamp");
                    
                    match sqlx::query!(
                        r#"
                        INSERT INTO access_tokens (token, org_id, expires_at)
                        VALUES ($1, $2, $3)
                        "#,
                        token,
                        record.id,
                        expires_at
                    )
                    .execute(&data.db_pool)
                    .await
                    {
                        Ok(_) => {
                            HttpResponse::Created().json(AuthResponse {
                                token,
                                organization,
                            })
                        },
                        Err(e) => {
                            eprintln!("Failed to store token: {}", e);
                            HttpResponse::InternalServerError().json(serde_json::json!({
                                "error": "Failed to create authentication token"
                            }))
                        }
                    }
                },
                Err(e) => {
                    eprintln!("Token generation error: {:?}", e);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Failed to generate authentication token"
                    }))
                }
            }
        },
        Err(e) => {
            eprintln!("Registration error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to register organization"
            }))
        }
    }
}

// Login handler
pub async fn login_organization(
    req: web::Json<LoginRequest>,
    data: web::Data<crate::AppState>,
) -> impl Responder {
    // Find organization by email
    match sqlx::query!(
        r#"
        SELECT id, name, email, password_hash, password_salt
        FROM organizations
        WHERE email = $1
        "#,
        req.email
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(record)) => {
            // Verify password
            if verify_password(&req.password, &record.password_hash) {
                let organization = Organization {
                    id: Some(record.id),
                    name: record.name,
                    email: record.email,
                };
                
                // Generate JWT token
                match generate_token(record.id) {
                    Ok(token) => {
                        // Store token in database
                        let expires_at = Utc::now()
                            .checked_add_signed(Duration::days(7))
                            .expect("valid timestamp");
                        
                        match sqlx::query!(
                            r#"
                            INSERT INTO access_tokens (token, org_id, expires_at)
                            VALUES ($1, $2, $3)
                            "#,
                            token,
                            record.id,
                            expires_at
                        )
                        .execute(&data.db_pool)
                        .await
                        {
                            Ok(_) => {
                                HttpResponse::Ok().json(AuthResponse {
                                    token,
                                    organization,
                                })
                            },
                            Err(e) => {
                                eprintln!("Failed to store token: {}", e);
                                HttpResponse::InternalServerError().json(serde_json::json!({
                                    "error": "Failed to create authentication token"
                                }))
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("Token generation error: {:?}", e);
                        HttpResponse::InternalServerError().json(serde_json::json!({
                            "error": "Failed to generate authentication token"
                        }))
                    }
                }
            } else {
                HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "Invalid credentials"
                }))
            }
        },
        Ok(None) => {
            // Organization not found
            HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Invalid credentials"
            }))
        },
        Err(e) => {
            eprintln!("Login error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to authenticate"
            }))
        }
    }
}

// Logout handler
pub async fn logout_organization(
    req: HttpRequest,
    data: web::Data<crate::AppState>,
) -> impl Responder {
    if let Some(token) = get_token_from_request(&req) {
        // Mark token as revoked
        match sqlx::query!(
            r#"
            UPDATE access_tokens
            SET is_revoked = true
            WHERE token = $1
            "#,
            token
        )
        .execute(&data.db_pool)
        .await
        {
            Ok(_) => HttpResponse::Ok().json(serde_json::json!({
                "message": "Logged out successfully"
            })),
            Err(e) => {
                eprintln!("Logout error: {}", e);
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to logout"
                }))
            }
        }
    } else {
        HttpResponse::BadRequest().json(serde_json::json!({
            "error": "No authentication token provided"
        }))
    }
}

// Middleware to verify tokens and extract organization ID
pub async fn verify_auth(
    req: &ServiceRequest,
    db_pool: &Pool<Postgres>,
) -> Result<i32, AuthError> {
    let token = match req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        Some(auth_header) if auth_header.starts_with("Bearer ") => {
            auth_header[7..].to_string()
        }
        _ => return Err(AuthError::Unauthorized("Missing or invalid authorization header".to_string())),
    };
    
    // Verify token is not revoked
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
    .map_err(AuthError::DatabaseError)?;
    
    match token_record {
        Some(record) => {
            if record.is_revoked {
                return Err(AuthError::Unauthorized("Token has been revoked".to_string()));
            }
            
            let expires_at = record.expires_at;
            let now = Utc::now().naive_utc();
            
            if expires_at < now {
                return Err(AuthError::Unauthorized("Token has expired".to_string()));
            }
            
            Ok(record.org_id)
        }
        None => Err(AuthError::Unauthorized("Invalid token".to_string())),
    }
}