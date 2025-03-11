use actix_web::{test, App, web};
use crud_hz_api::crud_hz_api_main::{configure_app, Organization};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use dotenv::dotenv;
use std::env;
use uuid::Uuid;

async fn setup_test_db() -> Pool<Postgres> {
    dotenv().ok();
    
    // Use a test-specific database or the regular one with a prefix for test data
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/postgres".to_string());
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create database connection pool");
    
    // Run migrations to ensure tables exist
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");
    
    pool
}

#[actix_rt::test]
async fn test_health_endpoint() {
    let db_pool = setup_test_db().await;
    
    let mut app = test::init_service(
        App::new().configure(|config| configure_app(config, db_pool.clone()))
    ).await;
    
    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&mut app, req).await;
    
    assert!(resp.status().is_success());
    
    let body = test::read_body(resp).await;
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(response["status"], "healthy");
}

#[actix_rt::test]
async fn test_create_organization() {
    let db_pool = setup_test_db().await;
    
    let mut app = test::init_service(
        App::new().configure(|config| configure_app(config, db_pool.clone()))
    ).await;
    
    // Generate unique data for the test
    let test_name = format!("Test Org {}", Uuid::new_v4());
    let test_email = format!("test{}@example.com", Uuid::new_v4());
    
    let org = Organization {
        id: None,
        name: test_name.clone(),
        email: test_email.clone(),
    };
    
    // Test creating an organization
    let req = test::TestRequest::post()
        .uri("/api/orgs")
        .set_json(&org)
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    
    assert!(resp.status().is_success());
    
    let body = test::read_body(resp).await;
    let created_org: Organization = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(created_org.name, test_name);
    assert_eq!(created_org.email, test_email);
    assert!(created_org.id.is_some());
    
    // Test getting the created organization
    let id = created_org.id.unwrap();
    let req = test::TestRequest::get()
        .uri(&format!("/api/orgs/{}", id))
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    
    assert!(resp.status().is_success());
    
    let body = test::read_body(resp).await;
    let fetched_org: Organization = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(fetched_org.id, created_org.id);
    assert_eq!(fetched_org.name, test_name);
    assert_eq!(fetched_org.email, test_email);
}

#[actix_rt::test]
async fn test_update_organization() {
    let db_pool = setup_test_db().await;
    
    let mut app = test::init_service(
        App::new().configure(|config| configure_app(config, db_pool.clone()))
    ).await;
    
    // Create an organization first
    let test_name = format!("Test Org {}", Uuid::new_v4());
    let test_email = format!("test{}@example.com", Uuid::new_v4());
    
    let org = Organization {
        id: None,
        name: test_name.clone(),
        email: test_email.clone(),
    };
    
    let req = test::TestRequest::post()
        .uri("/api/orgs")
        .set_json(&org)
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    let body = test::read_body(resp).await;
    let created_org: Organization = serde_json::from_slice(&body).unwrap();
    let id = created_org.id.unwrap();
    
    // Now update the organization
    let updated_name = format!("Updated Org {}", Uuid::new_v4());
    let updated_email = format!("updated{}@example.com", Uuid::new_v4());
    
    let updated_org = Organization {
        id: Some(id),
        name: updated_name.clone(),
        email: updated_email.clone(),
    };
    
    let req = test::TestRequest::put()
        .uri(&format!("/api/orgs/{}", id))
        .set_json(&updated_org)
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    
    assert!(resp.status().is_success());
    
    let body = test::read_body(resp).await;
    let resp_org: Organization = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(resp_org.id, Some(id));
    assert_eq!(resp_org.name, updated_name);
    assert_eq!(resp_org.email, updated_email);
}

#[actix_rt::test]
async fn test_delete_organization() {
    let db_pool = setup_test_db().await;
    
    let mut app = test::init_service(
        App::new().configure(|config| configure_app(config, db_pool.clone()))
    ).await;
    
    // Create an organization first
    let test_name = format!("Test Org {}", Uuid::new_v4());
    let test_email = format!("test{}@example.com", Uuid::new_v4());
    
    let org = Organization {
        id: None,
        name: test_name.clone(),
        email: test_email.clone(),
    };
    
    let req = test::TestRequest::post()
        .uri("/api/orgs")
        .set_json(&org)
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    let body = test::read_body(resp).await;
    let created_org: Organization = serde_json::from_slice(&body).unwrap();
    let id = created_org.id.unwrap();
    
    // Now delete the organization
    let req = test::TestRequest::delete()
        .uri(&format!("/api/orgs/{}", id))
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    
    assert_eq!(resp.status(), 204); // No Content
    
    // Verify it's deleted by trying to get it
    let req = test::TestRequest::get()
        .uri(&format!("/api/orgs/{}", id))
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    
    assert_eq!(resp.status(), 404); // Not Found
}