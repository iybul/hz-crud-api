use actix_web::{web, App, HttpResponse, HttpServer, Responder, middleware::Logger, HttpRequest, Error as ActixWebError};
use actix_cors::Cors;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
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
    date_made: NaiveDate,
    org_id: Option<i32>,
    ingredients: Vec<i32>,  // List of ingredient IDs
    description: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Ingredient {
    id: Option<i32>,
    lotcode: String,
    name: String,
    date: NaiveDate,
    org_id: Option<i32>,
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

// Employee endpoints
async fn create_employee(
    employee: web::Json<Employee>,
    data: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    // Get authenticated organization ID
    let auth_org_id = match get_org_id_from_token(&req, &data.db_pool).await {
        Ok(id) => id,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };
    
    // Enforce employee belongs to authenticated organization
    let org_id = auth_org_id;
    
    match sqlx::query!(
        "INSERT INTO employees (name, role, org_id) VALUES ($1, $2, $3) RETURNING id",
        employee.name,
        employee.role,
        org_id
    )
    .fetch_one(&data.db_pool)
    .await
    {
        Ok(record) => {
            let created_employee = Employee {
                id: Some(record.id),
                name: employee.name.clone(),
                role: employee.role.clone(),
                org_id,
            };
            HttpResponse::Created().json(created_employee)
        }
        Err(e) => {
            eprintln!("Failed to create employee: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create employee"}))
        }
    }
}

async fn get_employee(
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
    
    match sqlx::query!(
        "SELECT id, name, role, org_id FROM employees WHERE id = $1 AND org_id = $2",
        id,
        auth_org_id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(e)) => {
            let employee = Employee {
                id: Some(e.id),
                name: e.name,
                role: e.role,
                org_id: e.org_id.unwrap_or(auth_org_id),
            };
            HttpResponse::Ok().json(employee)
        },
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Employee not found"})),
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

async fn get_all_employees(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    // Get authenticated organization ID
    let auth_org_id = match get_org_id_from_token(&req, &data.db_pool).await {
        Ok(id) => id,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };
    
    match sqlx::query!(
        "SELECT id, name, role, org_id FROM employees WHERE org_id = $1",
        auth_org_id
    )
    .fetch_all(&data.db_pool)
    .await
    {
        Ok(employees) => {
            let result: Vec<Employee> = employees.into_iter()
                .map(|e| Employee {
                    id: Some(e.id),
                    name: e.name,
                    role: e.role,
                    org_id: e.org_id.unwrap_or(auth_org_id), // Convert Option<i32> to i32
                })
                .collect();
            HttpResponse::Ok().json(result)
        },
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

async fn update_employee(
    path: web::Path<i32>,
    employee: web::Json<Employee>,
    data: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    // Get authenticated organization ID
    let auth_org_id = match get_org_id_from_token(&req, &data.db_pool).await {
        Ok(id) => id,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };
    
    let id = path.into_inner();
    
    // Verify employee belongs to authenticated organization
    match sqlx::query!(
        "SELECT org_id FROM employees WHERE id = $1",
        id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(record)) => {
            if record.org_id != Some(auth_org_id) {
                return HttpResponse::Forbidden().json(serde_json::json!({"error": "Access denied"}));
            }
        },
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({"error": "Employee not found"}));
        },
        Err(e) => {
            eprintln!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    }
    
    // Update employee
    match sqlx::query!(
        "UPDATE employees SET name = $1, role = $2 WHERE id = $3 AND org_id = $4 RETURNING id",
        employee.name,
        employee.role,
        id,
        auth_org_id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(_)) => {
            let updated_employee = Employee {
                id: Some(id),
                name: employee.name.clone(),
                role: employee.role.clone(),
                org_id: auth_org_id,
            };
            HttpResponse::Ok().json(updated_employee)
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Employee not found"})),
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

async fn delete_employee(
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
    
    match sqlx::query!(
        "DELETE FROM employees WHERE id = $1 AND org_id = $2 RETURNING id", 
        id, 
        auth_org_id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(_)) => HttpResponse::NoContent().finish(),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Employee not found"})),
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

// Recipe endpoints
async fn create_recipe(
    recipe: web::Json<Recipe>,
    data: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    // Get authenticated organization ID
    let auth_org_id = match get_org_id_from_token(&req, &data.db_pool).await {
        Ok(id) => id,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };
    
    // Begin transaction
    let mut tx = match data.db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("Failed to begin transaction: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error"}));
        }
    };
    
    // Create recipe
    let recipe_id = match sqlx::query!(
        r#"
        INSERT INTO recipes (lotcode, name, date_made, org_id, description)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
        recipe.lotcode,
        recipe.name,
        recipe.date_made,
        auth_org_id,
        recipe.description
    )
    .fetch_one(&mut *tx)
    .await
    {
        Ok(record) => record.id,
        Err(e) => {
            let _ = tx.rollback().await;
            eprintln!("Failed to create recipe: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create recipe"}));
        }
    };
    
    // Add ingredients associations
    for ingredient_id in &recipe.ingredients {
        // Verify ingredient belongs to this organization
        match sqlx::query!(
            "SELECT id FROM ingredients WHERE id = $1 AND org_id = $2",
            ingredient_id,
            auth_org_id
        )
        .fetch_optional(&mut *tx)
        .await
        {
            Ok(Some(_)) => {
                // Ingredient belongs to this org, add association
                match sqlx::query!(
                    r#"
                    INSERT INTO recipe_ingredients (recipe_id, ingredient_id)
                    VALUES ($1, $2)
                    "#,
                    recipe_id,
                    ingredient_id
                )
                .execute(&mut *tx)
                .await
                {
                    Ok(_) => {},
                    Err(e) => {
                        let _ = tx.rollback().await;
                        eprintln!("Failed to associate ingredient: {}", e);
                        return HttpResponse::InternalServerError().json(serde_json::json!({
                            "error": "Failed to associate ingredient with recipe"
                        }));
                    }
                }
            },
            Ok(None) => {
                let _ = tx.rollback().await;
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "error": format!("Ingredient with ID {} not found or doesn't belong to your organization", ingredient_id)
                }));
            },
            Err(e) => {
                let _ = tx.rollback().await;
                eprintln!("Database error: {}", e);
                return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
            }
        }
    }
    
    // Commit transaction
    match tx.commit().await {
        Ok(_) => {
            let created_recipe = Recipe {
                id: Some(recipe_id),
                lotcode: recipe.lotcode.clone(),
                name: recipe.name.clone(),
                date_made: recipe.date_made,
                org_id: Some(auth_org_id),
                ingredients: recipe.ingredients.clone(),
                description: recipe.description.clone(),
            };
            HttpResponse::Created().json(created_recipe)
        },
        Err(e) => {
            eprintln!("Failed to commit transaction: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create recipe"}))
        }
    }
}

async fn get_recipe(
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
    
    // Get recipe
    let recipe = match sqlx::query!(
        r#"
        SELECT id, lotcode, name, date_made, org_id, description
        FROM recipes
        WHERE id = $1 AND org_id = $2
        "#,
        id,
        auth_org_id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(r)) => r,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({"error": "Recipe not found"})),
        Err(e) => {
            eprintln!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    };
    
    // Get ingredients
    let ingredients = match sqlx::query!(
        r#"
        SELECT ingredient_id
        FROM recipe_ingredients
        WHERE recipe_id = $1
        "#,
        recipe.id
    )
    .fetch_all(&data.db_pool)
    .await
    {
        Ok(records) => records.into_iter().map(|r| r.ingredient_id).collect(),
        Err(e) => {
            eprintln!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    };
    
    let result = Recipe {
        id: Some(recipe.id),
        lotcode: recipe.lotcode,
        name: recipe.name,
        date_made: recipe.date_made,
        org_id: recipe.org_id,
        ingredients,
        description: recipe.description,
    };
    
    HttpResponse::Ok().json(result)
}

async fn get_all_recipes(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    // Get authenticated organization ID
    let auth_org_id = match get_org_id_from_token(&req, &data.db_pool).await {
        Ok(id) => id,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };
    
    // Get all recipes for this organization
    let recipes = match sqlx::query!(
        r#"
        SELECT id, lotcode, name, date_made, org_id, description
        FROM recipes
        WHERE org_id = $1
        "#,
        auth_org_id
    )
    .fetch_all(&data.db_pool)
    .await
    {
        Ok(records) => records,
        Err(e) => {
            eprintln!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    };
    
    let mut result = Vec::new();
    
    // For each recipe, get its ingredients
    for recipe in recipes {
        let ingredients = match sqlx::query!(
            r#"
            SELECT ingredient_id
            FROM recipe_ingredients
            WHERE recipe_id = $1
            "#,
            recipe.id
        )
        .fetch_all(&data.db_pool)
        .await
        {
            Ok(records) => records.into_iter().map(|r| r.ingredient_id).collect(),
            Err(e) => {
                eprintln!("Database error: {}", e);
                return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
            }
        };
        
        result.push(Recipe {
            id: Some(recipe.id),
            lotcode: recipe.lotcode,
            name: recipe.name,
            date_made: recipe.date_made,
            org_id: recipe.org_id,
            ingredients,
            description: recipe.description,
        });
    }
    
    HttpResponse::Ok().json(result)
}

async fn update_recipe(
    path: web::Path<i32>,
    recipe: web::Json<Recipe>,
    data: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    // Get authenticated organization ID
    let auth_org_id = match get_org_id_from_token(&req, &data.db_pool).await {
        Ok(id) => id,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };
    
    let id = path.into_inner();
    
    // Verify recipe belongs to authenticated organization
    match sqlx::query!(
        "SELECT org_id FROM recipes WHERE id = $1",
        id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(record)) => {
            if record.org_id != Some(auth_org_id) {
                return HttpResponse::Forbidden().json(serde_json::json!({"error": "Access denied"}));
            }
        },
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({"error": "Recipe not found"}));
        },
        Err(e) => {
            eprintln!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    }
    
    // Begin transaction
    let mut tx = match data.db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("Failed to begin transaction: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error"}));
        }
    };
    
    // Update recipe
    match sqlx::query!(
        r#"
        UPDATE recipes 
        SET lotcode = $1, name = $2, date_made = $3, description = $4
        WHERE id = $5 AND org_id = $6
        RETURNING id
        "#,
        recipe.lotcode,
        recipe.name,
        recipe.date_made,
        recipe.description,
        id,
        auth_org_id
    )
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(_)) => {},
        Ok(None) => {
            let _ = tx.rollback().await;
            return HttpResponse::NotFound().json(serde_json::json!({"error": "Recipe not found"}));
        },
        Err(e) => {
            let _ = tx.rollback().await;
            eprintln!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    }
    
    // Delete existing ingredient associations
    match sqlx::query!(
        "DELETE FROM recipe_ingredients WHERE recipe_id = $1",
        id
    )
    .execute(&mut *tx)
    .await
    {
        Ok(_) => {},
        Err(e) => {
            let _ = tx.rollback().await;
            eprintln!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    }
    
    // Add new ingredient associations
    for ingredient_id in &recipe.ingredients {
        // Verify ingredient belongs to this organization
        match sqlx::query!(
            "SELECT id FROM ingredients WHERE id = $1 AND org_id = $2",
            ingredient_id,
            auth_org_id
        )
        .fetch_optional(&mut *tx)
        .await
        {
            Ok(Some(_)) => {
                // Ingredient belongs to this org, add association
                match sqlx::query!(
                    r#"
                    INSERT INTO recipe_ingredients (recipe_id, ingredient_id)
                    VALUES ($1, $2)
                    "#,
                    id,
                    ingredient_id
                )
                .execute(&mut *tx)
                .await
                {
                    Ok(_) => {},
                    Err(e) => {
                        let _ = tx.rollback().await;
                        eprintln!("Failed to associate ingredient: {}", e);
                        return HttpResponse::InternalServerError().json(serde_json::json!({
                            "error": "Failed to associate ingredient with recipe"
                        }));
                    }
                }
            },
            Ok(None) => {
                let _ = tx.rollback().await;
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "error": format!("Ingredient with ID {} not found or doesn't belong to your organization", ingredient_id)
                }));
            },
            Err(e) => {
                let _ = tx.rollback().await;
                eprintln!("Database error: {}", e);
                return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
            }
        }
    }
    
    // Commit transaction
    match tx.commit().await {
        Ok(_) => {
            let updated_recipe = Recipe {
                id: Some(id),
                lotcode: recipe.lotcode.clone(),
                name: recipe.name.clone(),
                date_made: recipe.date_made,
                org_id: Some(auth_org_id),
                ingredients: recipe.ingredients.clone(),
                description: recipe.description.clone(),
            };
            HttpResponse::Ok().json(updated_recipe)
        },
        Err(e) => {
            eprintln!("Failed to commit transaction: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to update recipe"}))
        }
    }
}

async fn delete_recipe(
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
    
    match sqlx::query!(
        "DELETE FROM recipes WHERE id = $1 AND org_id = $2 RETURNING id", 
        id, 
        auth_org_id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(_)) => HttpResponse::NoContent().finish(),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Recipe not found"})),
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

// Ingredient endpoints
async fn create_ingredient(
    ingredient: web::Json<Ingredient>,
    data: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    // Get authenticated organization ID
    let auth_org_id = match get_org_id_from_token(&req, &data.db_pool).await {
        Ok(id) => id,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };
    
    match sqlx::query!(
        r#"
        INSERT INTO ingredients (lotcode, name, date, org_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
        ingredient.lotcode,
        ingredient.name,
        ingredient.date,
        auth_org_id
    )
    .fetch_one(&data.db_pool)
    .await
    {
        Ok(record) => {
            let created_ingredient = Ingredient {
                id: Some(record.id),
                lotcode: ingredient.lotcode.clone(),
                name: ingredient.name.clone(),
                date: ingredient.date,
                org_id: Some(auth_org_id),
            };
            HttpResponse::Created().json(created_ingredient)
        }
        Err(e) => {
            eprintln!("Failed to create ingredient: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create ingredient"}))
        }
    }
}

async fn get_ingredient(
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
    
    match sqlx::query!(
        r#"
        SELECT id, lotcode, name, date, org_id
        FROM ingredients
        WHERE id = $1 AND org_id = $2
        "#,
        id,
        auth_org_id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(i)) => {
            let ingredient = Ingredient {
                id: Some(i.id),
                lotcode: i.lotcode,
                name: i.name,
                date: i.date,
                org_id: i.org_id,
            };
            HttpResponse::Ok().json(ingredient)
        },
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Ingredient not found"})),
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

async fn get_all_ingredients(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    // Get authenticated organization ID
    let auth_org_id = match get_org_id_from_token(&req, &data.db_pool).await {
        Ok(id) => id,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };
    
    match sqlx::query!(
        r#"
        SELECT id, lotcode, name, date, org_id
        FROM ingredients
        WHERE org_id = $1
        "#,
        auth_org_id
    )
    .fetch_all(&data.db_pool)
    .await
    {
        Ok(ingredients) => {
            let result: Vec<Ingredient> = ingredients.into_iter()
                .map(|i| Ingredient {
                    id: Some(i.id),
                    lotcode: i.lotcode,
                    name: i.name,
                    date: i.date,
                    org_id: i.org_id,
                })
                .collect();
            HttpResponse::Ok().json(result)
        },
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

async fn update_ingredient(
    path: web::Path<i32>,
    ingredient: web::Json<Ingredient>,
    data: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    // Get authenticated organization ID
    let auth_org_id = match get_org_id_from_token(&req, &data.db_pool).await {
        Ok(id) => id,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };
    
    let id = path.into_inner();
    
    // Verify ingredient belongs to authenticated organization
    match sqlx::query!(
        "SELECT org_id FROM ingredients WHERE id = $1",
        id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(record)) => {
            if record.org_id != Some(auth_org_id) {
                return HttpResponse::Forbidden().json(serde_json::json!({"error": "Access denied"}));
            }
        },
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({"error": "Ingredient not found"}));
        },
        Err(e) => {
            eprintln!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    }
    
    match sqlx::query!(
        r#"
        UPDATE ingredients 
        SET lotcode = $1, name = $2, date = $3
        WHERE id = $4 AND org_id = $5
        RETURNING id
        "#,
        ingredient.lotcode,
        ingredient.name,
        ingredient.date,
        id,
        auth_org_id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(_)) => {
            let updated_ingredient = Ingredient {
                id: Some(id),
                lotcode: ingredient.lotcode.clone(),
                name: ingredient.name.clone(),
                date: ingredient.date,
                org_id: Some(auth_org_id),
            };
            HttpResponse::Ok().json(updated_ingredient)
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Ingredient not found"})),
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

async fn delete_ingredient(
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
    
    match sqlx::query!(
        "DELETE FROM ingredients WHERE id = $1 AND org_id = $2 RETURNING id", 
        id, 
        auth_org_id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(_)) => HttpResponse::NoContent().finish(),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Ingredient not found"})),
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
                        // Employee endpoints
                        .service(
                            web::scope("/{org_id}/employees")
                                .route("", web::post().to(create_employee))
                                .route("", web::get().to(get_all_employees))
                                .route("/{id}", web::get().to(get_employee))
                                .route("/{id}", web::put().to(update_employee))
                                .route("/{id}", web::delete().to(delete_employee))
                        )
                )
                // Employee endpoints - direct access
                .service(
                    web::scope("/employees")
                        .route("", web::post().to(create_employee))
                        .route("", web::get().to(get_all_employees))
                        .route("/{id}", web::get().to(get_employee))
                        .route("/{id}", web::put().to(update_employee))
                        .route("/{id}", web::delete().to(delete_employee))
                )
                // Recipe endpoints
                .service(
                    web::scope("/recipes")
                        .route("", web::post().to(create_recipe))
                        .route("", web::get().to(get_all_recipes))
                        .route("/{id}", web::get().to(get_recipe))
                        .route("/{id}", web::put().to(update_recipe))
                        .route("/{id}", web::delete().to(delete_recipe))
                )
                // Ingredient endpoints
                .service(
                    web::scope("/ingredients")
                        .route("", web::post().to(create_ingredient))
                        .route("", web::get().to(get_all_ingredients))
                        .route("/{id}", web::get().to(get_ingredient))
                        .route("/{id}", web::put().to(update_ingredient))
                        .route("/{id}", web::delete().to(delete_ingredient))
                )
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