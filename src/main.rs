use actix_web::{web, App, HttpResponse, HttpServer, Responder, middleware::Logger};
use actix_cors::Cors;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, types::chrono::NaiveDate};
use std::env;

// Import auth module
mod auth;

// Organization entity
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Organization {
    pub id: Option<i32>,
    pub name: String,
    pub email: String,
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
    pub id: Option<i32>,
    pub lotcode: String,
    pub name: String,
    pub date_made: String,
    pub org_id: i32,
    pub ingredients: Vec<i32>,
    pub description: Option<String>,
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
    pub id: Option<i32>,
    pub lotcode: String,
    pub name: String,
    pub date: String,
    pub org_id: i32,
}

// Application state
pub struct AppState {
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
) -> impl Responder {
    let id = path.into_inner();
    
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
) -> impl Responder {
    match sqlx::query_as!(
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
) -> impl Responder {
    let id = path.into_inner();
    
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
) -> impl Responder {
    match sqlx::query!(
        "INSERT INTO employees (name, role, org_id) VALUES ($1, $2, $3) RETURNING id",
        employee.name,
        employee.role,
        employee.org_id
    )
    .fetch_one(&data.db_pool)
    .await
    {
        Ok(record) => {
            let created_employee = Employee {
                id: Some(record.id),
                name: employee.name.clone(),
                role: employee.role.clone(),
                org_id: employee.org_id,
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
) -> impl Responder {
    let id = path.into_inner();
    
    match sqlx::query!(
        "SELECT id, name, role, org_id FROM employees WHERE id = $1",
        id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(record)) => {
            let employee = Employee {
                id: Some(record.id),
                name: record.name,
                role: record.role,
                org_id: record.org_id.unwrap_or(0),
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
) -> impl Responder {
    match sqlx::query!(
        "SELECT id, name, role, org_id FROM employees"
    )
    .fetch_all(&data.db_pool)
    .await
    {
        Ok(records) => {
            let employees: Vec<Employee> = records.into_iter().map(|record| {
                Employee {
                    id: Some(record.id),
                    name: record.name,
                    role: record.role,
                    org_id: record.org_id.unwrap_or(0),
                }
            }).collect();
            HttpResponse::Ok().json(employees)
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
) -> impl Responder {
    let id = path.into_inner();
    
    match sqlx::query!(
        "UPDATE employees SET name = $1, role = $2, org_id = $3 WHERE id = $4 RETURNING id",
        employee.name,
        employee.role,
        employee.org_id,
        id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(_)) => {
            let updated_employee = Employee {
                id: Some(id),
                name: employee.name.clone(),
                role: employee.role.clone(),
                org_id: employee.org_id,
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
) -> impl Responder {
    let id = path.into_inner();
    
    match sqlx::query!("DELETE FROM employees WHERE id = $1 RETURNING id", id)
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

// Ingredient endpoints
async fn create_ingredient(
    ingredient: web::Json<IngredientInput>,
    data: web::Data<AppState>,
) -> impl Responder {
    // Parse the date string to NaiveDate
    let date = match NaiveDate::parse_from_str(&ingredient.date, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid date format. Use YYYY-MM-DD"
            }));
        }
    };

    match sqlx::query!(
        "INSERT INTO ingredients (lotcode, name, date, org_id) VALUES ($1, $2, $3, $4) RETURNING id",
        ingredient.lotcode,
        ingredient.name,
        date,
        ingredient.org_id
    )
    .fetch_one(&data.db_pool)
    .await
    {
        Ok(record) => {
            let created_ingredient = Ingredient {
                id: Some(record.id),
                lotcode: ingredient.lotcode.clone(),
                name: ingredient.name.clone(),
                date: ingredient.date.clone(),
                org_id: ingredient.org_id,
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
) -> impl Responder {
    let id = path.into_inner();
    
    match sqlx::query!(
        "SELECT id, lotcode, name, date::text as date, org_id FROM ingredients WHERE id = $1",
        id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(record)) => {
            let ingredient = Ingredient {
                id: Some(record.id),
                lotcode: record.lotcode,
                name: record.name,
                date: record.date.unwrap_or_default(),
                org_id: record.org_id.unwrap_or(0),
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
) -> impl Responder {
    match sqlx::query!(
        "SELECT id, lotcode, name, date::text as date, org_id FROM ingredients"
    )
    .fetch_all(&data.db_pool)
    .await
    {
        Ok(records) => {
            let ingredients: Vec<Ingredient> = records.into_iter().map(|record| {
                Ingredient {
                    id: Some(record.id),
                    lotcode: record.lotcode,
                    name: record.name,
                    date: record.date.unwrap_or_default(),
                    org_id: record.org_id.unwrap_or(0),
                }
            }).collect();
            HttpResponse::Ok().json(ingredients)
        },
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

async fn update_ingredient(
    path: web::Path<i32>,
    ingredient: web::Json<IngredientInput>,
    data: web::Data<AppState>,
) -> impl Responder {
    let id = path.into_inner();
    
    // Parse the date string to NaiveDate
    let date = match NaiveDate::parse_from_str(&ingredient.date, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid date format. Use YYYY-MM-DD"
            }));
        }
    };
    
    match sqlx::query!(
        "UPDATE ingredients SET lotcode = $1, name = $2, date = $3, org_id = $4 WHERE id = $5 RETURNING id",
        ingredient.lotcode,
        ingredient.name,
        date,
        ingredient.org_id,
        id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(_)) => {
            let updated_ingredient = Ingredient {
                id: Some(id),
                lotcode: ingredient.lotcode.clone(),
                name: ingredient.name.clone(),
                date: ingredient.date.clone(),
                org_id: ingredient.org_id,
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
) -> impl Responder {
    let id = path.into_inner();
    
    match sqlx::query!("DELETE FROM ingredients WHERE id = $1 RETURNING id", id)
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

// Recipe endpoints
async fn create_recipe(
    recipe: web::Json<RecipeInput>,
    data: web::Data<AppState>,
) -> impl Responder {
    // Parse the date string to NaiveDate
    let date_made = match NaiveDate::parse_from_str(&recipe.date_made, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid date format. Use YYYY-MM-DD"
            }));
        }
    };

    // Start a transaction
    let mut tx = match data.db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("Failed to start transaction: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    };

    // Insert the recipe
    let recipe_id = match sqlx::query!(
        "INSERT INTO recipes (lotcode, name, date_made, org_id, description) VALUES ($1, $2, $3, $4, $5) RETURNING id",
        recipe.lotcode,
        recipe.name,
        date_made,
        recipe.org_id,
        recipe.description
    )
    .fetch_one(&mut *tx)
    .await
    {
        Ok(record) => record.id,
        Err(e) => {
            eprintln!("Failed to create recipe: {}", e);
            let _ = tx.rollback().await;
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create recipe"}));
        }
    };

    // Insert recipe-ingredient relationships
    for ingredient_id in &recipe.ingredients {
        if let Err(e) = sqlx::query!(
            "INSERT INTO recipe_ingredients (recipe_id, ingredient_id) VALUES ($1, $2)",
            recipe_id,
            ingredient_id
        )
        .execute(&mut *tx)
        .await
        {
            eprintln!("Failed to link ingredient to recipe: {}", e);
            let _ = tx.rollback().await;
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create recipe"}));
        }
    }

    // Commit the transaction
    if let Err(e) = tx.commit().await {
        eprintln!("Failed to commit transaction: {}", e);
        return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
    }

    // Return the created recipe
    let created_recipe = Recipe {
        id: Some(recipe_id),
        lotcode: recipe.lotcode.clone(),
        name: recipe.name.clone(),
        date_made: recipe.date_made.clone(),
        org_id: recipe.org_id,
        ingredients: recipe.ingredients.clone(),
        description: Some(recipe.description.clone()),
    };

    HttpResponse::Created().json(created_recipe)
}

async fn get_recipe(
    path: web::Path<i32>,
    data: web::Data<AppState>,
) -> impl Responder {
    let id = path.into_inner();
    
    // Get the recipe
    match sqlx::query!(
        "SELECT id, lotcode, name, date_made::text as date_made, org_id, description, 
         ARRAY(SELECT ingredient_id FROM recipe_ingredients WHERE recipe_id = $1)::int[] as ingredients 
         FROM recipes WHERE id = $1",
        id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(record)) => {
            let recipe = Recipe {
                id: Some(record.id),
                lotcode: record.lotcode,
                name: record.name,
                date_made: record.date_made.unwrap_or_default(),
                org_id: record.org_id.unwrap_or(0),
                ingredients: record.ingredients.unwrap_or_default(),
                description: record.description,
            };
            HttpResponse::Ok().json(recipe)
        },
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Recipe not found"})),
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

async fn get_all_recipes(
    data: web::Data<AppState>,
) -> impl Responder {
    match sqlx::query!(
        "SELECT r.id, r.lotcode, r.name, r.date_made::text as date_made, r.org_id, r.description,
         ARRAY(SELECT ingredient_id FROM recipe_ingredients WHERE recipe_id = r.id)::int[] as ingredients
         FROM recipes r"
    )
    .fetch_all(&data.db_pool)
    .await
    {
        Ok(records) => {
            let recipes: Vec<Recipe> = records.into_iter().map(|record| {
                Recipe {
                    id: Some(record.id),
                    lotcode: record.lotcode,
                    name: record.name,
                    date_made: record.date_made.unwrap_or_default(),
                    org_id: record.org_id.unwrap_or(0),
                    ingredients: record.ingredients.unwrap_or_default(),
                    description: record.description,
                }
            }).collect();
            HttpResponse::Ok().json(recipes)
        },
        Err(e) => {
            eprintln!("Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

async fn update_recipe(
    path: web::Path<i32>,
    recipe: web::Json<RecipeInput>,
    data: web::Data<AppState>,
) -> impl Responder {
    let id = path.into_inner();
    
    // Parse the date string to NaiveDate
    let date_made = match NaiveDate::parse_from_str(&recipe.date_made, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid date format. Use YYYY-MM-DD"
            }));
        }
    };
    
    // Start a transaction
    let mut tx = match data.db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("Failed to start transaction: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    };

    // Update the recipe
    let update_result = sqlx::query!(
        "UPDATE recipes SET lotcode = $1, name = $2, date_made = $3, org_id = $4, description = $5 
         WHERE id = $6 RETURNING id",
        recipe.lotcode,
        recipe.name,
        date_made,
        recipe.org_id,
        recipe.description,
        id
    )
    .fetch_optional(&mut *tx)
    .await;

    match update_result {
        Ok(Some(_)) => {
            // Delete existing recipe-ingredient relationships
            if let Err(e) = sqlx::query!("DELETE FROM recipe_ingredients WHERE recipe_id = $1", id)
                .execute(&mut *tx)
                .await
            {
                eprintln!("Failed to delete ingredient relations: {}", e);
                let _ = tx.rollback().await;
                return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to update recipe"}));
            }

            // Insert new recipe-ingredient relationships
            for ingredient_id in &recipe.ingredients {
                if let Err(e) = sqlx::query!(
                    "INSERT INTO recipe_ingredients (recipe_id, ingredient_id) VALUES ($1, $2)",
                    id,
                    ingredient_id
                )
                .execute(&mut *tx)
                .await
                {
                    eprintln!("Failed to link ingredient to recipe: {}", e);
                    let _ = tx.rollback().await;
                    return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to update recipe"}));
                }
            }

            // Commit the transaction
            if let Err(e) = tx.commit().await {
                eprintln!("Failed to commit transaction: {}", e);
                return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
            }

            // Return the updated recipe
            let updated_recipe = Recipe {
                id: Some(id),
                lotcode: recipe.lotcode.clone(),
                name: recipe.name.clone(),
                date_made: recipe.date_made.clone(),
                org_id: recipe.org_id,
                ingredients: recipe.ingredients.clone(),
                description: Some(recipe.description.clone()),
            };
            HttpResponse::Ok().json(updated_recipe)
        },
        Ok(None) => {
            let _ = tx.rollback().await;
            HttpResponse::NotFound().json(serde_json::json!({"error": "Recipe not found"}))
        },
        Err(e) => {
            eprintln!("Database error: {}", e);
            let _ = tx.rollback().await;
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

async fn delete_recipe(
    path: web::Path<i32>,
    data: web::Data<AppState>,
) -> impl Responder {
    let id = path.into_inner();
    
    // Start a transaction
    let mut tx = match data.db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("Failed to start transaction: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    };

    // Delete recipe-ingredient relationships first
    if let Err(e) = sqlx::query!("DELETE FROM recipe_ingredients WHERE recipe_id = $1", id)
        .execute(&mut *tx)
        .await
    {
        eprintln!("Failed to delete ingredient relations: {}", e);
        let _ = tx.rollback().await;
        return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to delete recipe"}));
    }

    // Delete the recipe
    match sqlx::query!("DELETE FROM recipes WHERE id = $1 RETURNING id", id)
        .fetch_optional(&mut *tx)
        .await
    {
        Ok(Some(_)) => {
            // Commit the transaction
            if let Err(e) = tx.commit().await {
                eprintln!("Failed to commit transaction: {}", e);
                return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
            }
            HttpResponse::NoContent().finish()
        },
        Ok(None) => {
            let _ = tx.rollback().await;
            HttpResponse::NotFound().json(serde_json::json!({"error": "Recipe not found"}))
        },
        Err(e) => {
            eprintln!("Database error: {}", e);
            let _ = tx.rollback().await;
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
                // Employee endpoints
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