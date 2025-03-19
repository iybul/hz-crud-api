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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProblemLog {
    pub id: Option<i32>,
    #[serde(alias = "isOpen")]
    pub is_open: bool,
    #[serde(alias = "dateOpened")]
    pub date_opened: String,
    #[serde(alias = "customerName")]
    pub customer_name: String,
    #[serde(alias = "problemType")]
    pub problem_type: String,
    #[serde(alias = "assignedTo")]
    pub assigned_to: Vec<i32>,
    #[serde(alias = "problemDescription")]
    pub problem_description: String,
    pub recall: bool,
    #[serde(alias = "dateResolved")]
    pub date_resolved: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProblemLogInput {
    #[serde(alias = "isOpen")]
    pub is_open: bool,
    #[serde(alias = "dateOpened")]
    pub date_opened: String,
    #[serde(alias = "customerName")]
    pub customer_name: String,
    #[serde(alias = "problemType")]
    pub problem_type: String,
    #[serde(alias = "assignedTo")]
    pub assigned_to: Vec<i32>,
    #[serde(alias = "problemDescription")]
    pub problem_description: String,
    pub recall: bool,
    #[serde(alias = "dateResolved")]
    pub date_resolved: Option<String>,
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

// Batch endpoints
async fn create_batch(
    batch: web::Json<BatchInput>,
    data: web::Data<AppState>,
) -> impl Responder {
    // Parse the date string to NaiveDate
    let date_made = match NaiveDate::parse_from_str(&batch.date_made, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid date format. Use YYYY-MM-DD"
            }));
        }
    };

    // Validate ingredients and amounts have the same length
    if batch.ingredients.len() != batch.amount_ingredients.len() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Ingredients and amounts must have the same length"
        }));
    }

    // Start a transaction
    let mut tx = match data.db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("Failed to start transaction: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    };

    // Insert the batch
    let batch_id = match sqlx::query!(
        "INSERT INTO batches (org_id, employee, recipe_lotcode, batch_lot_code, date_made, amount_made) 
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        batch.org_id,
        batch.employee,
        batch.recipe_lotcode,
        batch.batch_lot_code,
        date_made,
        batch.amount_made
    )
    .fetch_one(&mut *tx)
    .await
    {
        Ok(record) => record.id,
        Err(e) => {
            eprintln!("Failed to create batch: {}", e);
            let _ = tx.rollback().await;
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create batch"}));
        }
    };

    // Insert batch-ingredient relationships with amounts
    for (index, ingredient_id) in batch.ingredients.iter().enumerate() {
        if let Err(e) = sqlx::query!(
            "INSERT INTO batch_ingredients (batch_id, ingredient_id, amount) VALUES ($1, $2, $3)",
            batch_id,
            ingredient_id,
            batch.amount_ingredients[index]
        )
        .execute(&mut *tx)
        .await
        {
            eprintln!("Failed to link ingredient to batch: {}", e);
            let _ = tx.rollback().await;
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create batch"}));
        }
    }

    // Commit the transaction
    if let Err(e) = tx.commit().await {
        eprintln!("Failed to commit transaction: {}", e);
        return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
    }

    // Return the created batch
    let created_batch = Batch {
        id: Some(batch_id),
        org_id: batch.org_id,
        employee: batch.employee.clone(),
        recipe_lotcode: batch.recipe_lotcode.clone(),
        batch_lot_code: batch.batch_lot_code.clone(),
        ingredients: batch.ingredients.clone(),
        amount_ingredients: batch.amount_ingredients.clone(),
        date_made: batch.date_made.clone(),
        amount_made: batch.amount_made.clone(),
    };

    HttpResponse::Created().json(created_batch)
}

async fn get_batch(
    path: web::Path<i32>,
    data: web::Data<AppState>,
) -> impl Responder {
    let id = path.into_inner();
    
    // Get the basic batch information
    let batch_record = match sqlx::query!(
        "SELECT id, org_id, employee, recipe_lotcode, batch_lot_code, date_made::text as date_made, amount_made 
         FROM batches WHERE id = $1",
        id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(record)) => record,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({"error": "Batch not found"})),
        Err(e) => {
            eprintln!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    };

    // Get the batch ingredients with amounts
    let batch_ingredients = match sqlx::query!(
        "SELECT bi.ingredient_id, bi.amount 
         FROM batch_ingredients bi 
         WHERE bi.batch_id = $1",
        id
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

    // Extract ingredient ids and amounts
    let ingredients: Vec<i32> = batch_ingredients.iter().map(|r| r.ingredient_id).collect();
    let amount_ingredients: Vec<i32> = batch_ingredients.iter().map(|r| r.amount).collect();

    // Create the complete batch object
    let batch = Batch {
        id: Some(batch_record.id),
        org_id: batch_record.org_id.unwrap_or(0),
        employee: batch_record.employee,
        recipe_lotcode: batch_record.recipe_lotcode,
        batch_lot_code: batch_record.batch_lot_code,
        ingredients,
        amount_ingredients,
        date_made: batch_record.date_made.unwrap_or_default(),
        amount_made: batch_record.amount_made,
    };

    HttpResponse::Ok().json(batch)
}

async fn get_all_batches(
    data: web::Data<AppState>,
) -> impl Responder {
    // Get all batches
    let batch_records = match sqlx::query!(
        "SELECT id, org_id, employee, recipe_lotcode, batch_lot_code, date_made::text as date_made, amount_made 
         FROM batches ORDER BY id DESC"
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

    // Create a vector to hold all batches
    let mut batches = Vec::new();

    // For each batch, get its ingredients and amounts
    for record in batch_records {
        let batch_id = record.id;

        // Get the batch ingredients with amounts
        let batch_ingredients = match sqlx::query!(
            "SELECT bi.ingredient_id, bi.amount 
             FROM batch_ingredients bi 
             WHERE bi.batch_id = $1",
            batch_id
        )
        .fetch_all(&data.db_pool)
        .await
        {
            Ok(records) => records,
            Err(e) => {
                eprintln!("Database error when fetching ingredients for batch {}: {}", batch_id, e);
                continue; // Skip this batch if we can't get its ingredients
            }
        };

        // Extract ingredient ids and amounts
        let ingredients: Vec<i32> = batch_ingredients.iter().map(|r| r.ingredient_id).collect();
        let amount_ingredients: Vec<i32> = batch_ingredients.iter().map(|r| r.amount).collect();

        // Create the complete batch object
        let batch = Batch {
            id: Some(record.id),
            org_id: record.org_id.unwrap_or(0),
            employee: record.employee,
            recipe_lotcode: record.recipe_lotcode,
            batch_lot_code: record.batch_lot_code,
            ingredients,
            amount_ingredients,
            date_made: record.date_made.unwrap_or_default(),
            amount_made: record.amount_made,
        };

        batches.push(batch);
    }

    HttpResponse::Ok().json(batches)
}

async fn update_batch(
    path: web::Path<i32>,
    batch: web::Json<BatchInput>,
    data: web::Data<AppState>,
) -> impl Responder {
    let id = path.into_inner();
    
    // Parse the date string to NaiveDate
    let date_made = match NaiveDate::parse_from_str(&batch.date_made, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid date format. Use YYYY-MM-DD"
            }));
        }
    };

    // Validate ingredients and amounts have the same length
    if batch.ingredients.len() != batch.amount_ingredients.len() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Ingredients and amounts must have the same length"
        }));
    }

    // Start a transaction
    let mut tx = match data.db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("Failed to start transaction: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    };

    // Update the batch
    let update_result = sqlx::query!(
        "UPDATE batches SET org_id = $1, employee = $2, recipe_lotcode = $3, batch_lot_code = $4, 
         date_made = $5, amount_made = $6 WHERE id = $7 RETURNING id",
        batch.org_id,
        batch.employee,
        batch.recipe_lotcode,
        batch.batch_lot_code,
        date_made,
        batch.amount_made,
        id
    )
    .fetch_optional(&mut *tx)
    .await;

    match update_result {
        Ok(Some(_)) => {
            // Delete existing batch-ingredient relationships
            if let Err(e) = sqlx::query!("DELETE FROM batch_ingredients WHERE batch_id = $1", id)
                .execute(&mut *tx)
                .await
            {
                eprintln!("Failed to delete ingredient relations: {}", e);
                let _ = tx.rollback().await;
                return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to update batch"}));
            }

            // Insert new batch-ingredient relationships with amounts
            for (index, ingredient_id) in batch.ingredients.iter().enumerate() {
                if let Err(e) = sqlx::query!(
                    "INSERT INTO batch_ingredients (batch_id, ingredient_id, amount) VALUES ($1, $2, $3)",
                    id,
                    ingredient_id,
                    batch.amount_ingredients[index]
                )
                .execute(&mut *tx)
                .await
                {
                    eprintln!("Failed to link ingredient to batch: {}", e);
                    let _ = tx.rollback().await;
                    return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to update batch"}));
                }
            }

            // Commit the transaction
            if let Err(e) = tx.commit().await {
                eprintln!("Failed to commit transaction: {}", e);
                return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
            }

            // Return the updated batch
            let updated_batch = Batch {
                id: Some(id),
                org_id: batch.org_id,
                employee: batch.employee.clone(),
                recipe_lotcode: batch.recipe_lotcode.clone(),
                batch_lot_code: batch.batch_lot_code.clone(),
                ingredients: batch.ingredients.clone(),
                amount_ingredients: batch.amount_ingredients.clone(),
                date_made: batch.date_made.clone(),
                amount_made: batch.amount_made.clone(),
            };
            HttpResponse::Ok().json(updated_batch)
        },
        Ok(None) => {
            let _ = tx.rollback().await;
            HttpResponse::NotFound().json(serde_json::json!({"error": "Batch not found"}))
        },
        Err(e) => {
            eprintln!("Database error: {}", e);
            let _ = tx.rollback().await;
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

async fn delete_batch(
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

    // Delete batch-ingredient relationships first
    if let Err(e) = sqlx::query!("DELETE FROM batch_ingredients WHERE batch_id = $1", id)
        .execute(&mut *tx)
        .await
    {
        eprintln!("Failed to delete ingredient relations: {}", e);
        let _ = tx.rollback().await;
        return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to delete batch"}));
    }

    // Delete the batch
    match sqlx::query!("DELETE FROM batches WHERE id = $1 RETURNING id", id)
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
            HttpResponse::NotFound().json(serde_json::json!({"error": "Batch not found"}))
        },
        Err(e) => {
            eprintln!("Database error: {}", e);
            let _ = tx.rollback().await;
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

// Problem Log endpoints
async fn create_problem_log(
    problem_log: web::Json<ProblemLogInput>,
    data: web::Data<AppState>,
) -> impl Responder {
    // Parse date_opened string to NaiveDate
    let date_opened = match NaiveDate::parse_from_str(&problem_log.date_opened, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid date_opened format. Use YYYY-MM-DD"
            }));
        }
    };

    // Parse date_resolved string to NaiveDate if it exists
    let date_resolved = match &problem_log.date_resolved {
        Some(date_str) => {
            match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                Ok(date) => Some(date),
                Err(_) => {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "error": "Invalid date_resolved format. Use YYYY-MM-DD"
                    }));
                }
            }
        },
        None => None
    };

    // Start a transaction
    let mut tx = match data.db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("Failed to start transaction: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    };

    // Insert the problem log
    let problem_log_id = match sqlx::query!(
        "INSERT INTO problem_logs (is_open, date_opened, customer_name, problem_type, problem_description, recall, date_resolved) 
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        problem_log.is_open,
        date_opened,
        problem_log.customer_name,
        problem_log.problem_type,
        problem_log.problem_description,
        problem_log.recall,
        date_resolved
    )
    .fetch_one(&mut *tx)
    .await
    {
        Ok(record) => record.id,
        Err(e) => {
            eprintln!("Failed to create problem log: {}", e);
            let _ = tx.rollback().await;
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create problem log"}));
        }
    };

    // Insert problem log-employee relationships
    for employee_id in &problem_log.assigned_to {
        if let Err(e) = sqlx::query!(
            "INSERT INTO problem_logs_employees (problem_log_id, employee_id) VALUES ($1, $2)",
            problem_log_id,
            employee_id
        )
        .execute(&mut *tx)
        .await
        {
            eprintln!("Failed to link employee to problem log: {}", e);
            let _ = tx.rollback().await;
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create problem log"}));
        }
    }

    // Commit the transaction
    if let Err(e) = tx.commit().await {
        eprintln!("Failed to commit transaction: {}", e);
        return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
    }

    // Return the created problem log
    let created_problem_log = ProblemLog {
        id: Some(problem_log_id),
        is_open: problem_log.is_open,
        date_opened: problem_log.date_opened.clone(),
        customer_name: problem_log.customer_name.clone(),
        problem_type: problem_log.problem_type.clone(),
        assigned_to: problem_log.assigned_to.clone(),
        problem_description: problem_log.problem_description.clone(),
        recall: problem_log.recall,
        date_resolved: problem_log.date_resolved.clone(),
    };

    HttpResponse::Created().json(created_problem_log)
}

async fn get_problem_log(
    path: web::Path<i32>,
    data: web::Data<AppState>,
) -> impl Responder {
    let id = path.into_inner();
    
    // Get the problem log
    let problem_log_record = match sqlx::query!(
        "SELECT id, is_open, date_opened::text as date_opened, customer_name, problem_type, problem_description, 
         recall, date_resolved::text as date_resolved FROM problem_logs WHERE id = $1",
        id
    )
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(Some(record)) => record,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({"error": "Problem log not found"})),
        Err(e) => {
            eprintln!("Database error: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    };

    // Get the assigned employees
    let assigned_employees = match sqlx::query!(
        "SELECT employee_id FROM problem_logs_employees WHERE problem_log_id = $1",
        id
    )
    .fetch_all(&data.db_pool)
    .await
    {
        Ok(records) => records.into_iter().map(|r| r.employee_id).collect(),
        Err(e) => {
            eprintln!("Database error when fetching assigned employees: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    };

    // Create the complete problem log object
    let problem_log = ProblemLog {
        id: Some(problem_log_record.id),
        is_open: problem_log_record.is_open,
        date_opened: problem_log_record.date_opened.unwrap_or_default(),
        customer_name: problem_log_record.customer_name,
        problem_type: problem_log_record.problem_type,
        assigned_to: assigned_employees,
        problem_description: problem_log_record.problem_description,
        recall: problem_log_record.recall,
        date_resolved: problem_log_record.date_resolved,
    };

    HttpResponse::Ok().json(problem_log)
}

async fn get_all_problem_logs(
    data: web::Data<AppState>,
) -> impl Responder {
    // Get all problem logs
    let problem_log_records = match sqlx::query!(
        "SELECT id, is_open, date_opened::text as date_opened, customer_name, problem_type, problem_description, 
         recall, date_resolved::text as date_resolved FROM problem_logs ORDER BY id DESC"
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

    // Create a vector to hold all problem logs
    let mut problem_logs = Vec::new();

    // For each problem log, get its assigned employees
    for record in problem_log_records {
        let problem_log_id = record.id;

        // Get the assigned employees
        let assigned_employees = match sqlx::query!(
            "SELECT employee_id FROM problem_logs_employees WHERE problem_log_id = $1",
            problem_log_id
        )
        .fetch_all(&data.db_pool)
        .await
        {
            Ok(records) => records.into_iter().map(|r| r.employee_id).collect(),
            Err(e) => {
                eprintln!("Database error when fetching assigned employees for problem log {}: {}", problem_log_id, e);
                continue; // Skip this problem log if we can't get its assigned employees
            }
        };

        // Create the complete problem log object
        let problem_log = ProblemLog {
            id: Some(record.id),
            is_open: record.is_open,
            date_opened: record.date_opened.unwrap_or_default(),
            customer_name: record.customer_name,
            problem_type: record.problem_type,
            assigned_to: assigned_employees,
            problem_description: record.problem_description,
            recall: record.recall,
            date_resolved: record.date_resolved,
        };

        problem_logs.push(problem_log);
    }

    HttpResponse::Ok().json(problem_logs)
}

async fn update_problem_log(
    path: web::Path<i32>,
    problem_log: web::Json<ProblemLogInput>,
    data: web::Data<AppState>,
) -> impl Responder {
    let id = path.into_inner();
    
    // Parse date_opened string to NaiveDate
    let date_opened = match NaiveDate::parse_from_str(&problem_log.date_opened, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid date_opened format. Use YYYY-MM-DD"
            }));
        }
    };

    // Parse date_resolved string to NaiveDate if it exists
    let date_resolved = match &problem_log.date_resolved {
        Some(date_str) => {
            match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                Ok(date) => Some(date),
                Err(_) => {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "error": "Invalid date_resolved format. Use YYYY-MM-DD"
                    }));
                }
            }
        },
        None => None
    };

    // Start a transaction
    let mut tx = match data.db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("Failed to start transaction: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
        }
    };

    // Update the problem log
    let update_result = sqlx::query!(
        "UPDATE problem_logs SET is_open = $1, date_opened = $2, customer_name = $3, problem_type = $4, 
         problem_description = $5, recall = $6, date_resolved = $7 WHERE id = $8 RETURNING id",
        problem_log.is_open,
        date_opened,
        problem_log.customer_name,
        problem_log.problem_type,
        problem_log.problem_description,
        problem_log.recall,
        date_resolved,
        id
    )
    .fetch_optional(&mut *tx)
    .await;

    match update_result {
        Ok(Some(_)) => {
            // Delete existing problem log-employee relationships
            if let Err(e) = sqlx::query!("DELETE FROM problem_logs_employees WHERE problem_log_id = $1", id)
                .execute(&mut *tx)
                .await
            {
                eprintln!("Failed to delete employee relations: {}", e);
                let _ = tx.rollback().await;
                return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to update problem log"}));
            }

            // Insert new problem log-employee relationships
            for employee_id in &problem_log.assigned_to {
                if let Err(e) = sqlx::query!(
                    "INSERT INTO problem_logs_employees (problem_log_id, employee_id) VALUES ($1, $2)",
                    id,
                    employee_id
                )
                .execute(&mut *tx)
                .await
                {
                    eprintln!("Failed to link employee to problem log: {}", e);
                    let _ = tx.rollback().await;
                    return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to update problem log"}));
                }
            }

            // Commit the transaction
            if let Err(e) = tx.commit().await {
                eprintln!("Failed to commit transaction: {}", e);
                return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}));
            }

            // Return the updated problem log
            let updated_problem_log = ProblemLog {
                id: Some(id),
                is_open: problem_log.is_open,
                date_opened: problem_log.date_opened.clone(),
                customer_name: problem_log.customer_name.clone(),
                problem_type: problem_log.problem_type.clone(),
                assigned_to: problem_log.assigned_to.clone(),
                problem_description: problem_log.problem_description.clone(),
                recall: problem_log.recall,
                date_resolved: problem_log.date_resolved.clone(),
            };
            HttpResponse::Ok().json(updated_problem_log)
        },
        Ok(None) => {
            let _ = tx.rollback().await;
            HttpResponse::NotFound().json(serde_json::json!({"error": "Problem log not found"}))
        },
        Err(e) => {
            eprintln!("Database error: {}", e);
            let _ = tx.rollback().await;
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Internal server error"}))
        }
    }
}

async fn delete_problem_log(
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

    // Delete problem log-employee relationships first
    if let Err(e) = sqlx::query!("DELETE FROM problem_logs_employees WHERE problem_log_id = $1", id)
        .execute(&mut *tx)
        .await
    {
        eprintln!("Failed to delete employee relations: {}", e);
        let _ = tx.rollback().await;
        return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to delete problem log"}));
    }

    // Delete the problem log
    match sqlx::query!("DELETE FROM problem_logs WHERE id = $1 RETURNING id", id)
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
            HttpResponse::NotFound().json(serde_json::json!({"error": "Problem log not found"}))
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
                // Batch endpoints
                .service(
                    web::scope("/batches")
                        .route("", web::post().to(create_batch))
                        .route("", web::get().to(get_all_batches))
                        .route("/{id}", web::get().to(get_batch))
                        .route("/{id}", web::put().to(update_batch))
                        .route("/{id}", web::delete().to(delete_batch))
                )
                // Problem Log endpoints
                .service(
                    web::scope("/problemlogs")
                        .route("", web::post().to(create_problem_log))
                        .route("", web::get().to(get_all_problem_logs))
                        .route("/{id}", web::get().to(get_problem_log))
                        .route("/{id}", web::put().to(update_problem_log))
                        .route("/{id}", web::delete().to(delete_problem_log))
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