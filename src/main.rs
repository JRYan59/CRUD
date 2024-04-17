use std::fmt;
use actix_web::{web, App, HttpResponse, HttpServer, Responder, ResponseError};
use tokio_postgres::{Client, NoTls, Error};
use std::sync::{Arc, Mutex};
use serde::Deserialize;

#[derive(Debug)]
struct MyError(String);

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ResponseError for MyError {}

async fn connect() -> Result<Client, Error> {
    let (client, connection) = tokio_postgres::connect(
        "host=localhost user=postgres password=password dbname=prueba1",
        NoTls,
    ).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    Ok(client)
}

async fn get_items(db: web::Data<Arc<Mutex<Client>>>) -> Result<impl Responder, MyError> {
    let rows = db.lock().unwrap().query("SELECT id, name, description FROM items", &[]).await
        .map_err(|e| MyError(format!("Database error: {}", e)))?;
    let mut result = String::new();
    for row in &rows {
        let id: i32 = row.get(0);
        let name: &str = row.get(1);
        let description: &str = row.get(2);
        result.push_str(&format!("id: {}, name: {}, description: {}\n", id, name, description));
    }
    Ok(result)
}

#[derive(Debug, Deserialize)]
struct ItemData {
    name: String,
    description: String,
}

async fn create_item(
    db: web::Data<Arc<Mutex<Client>>>,
    item_data: web::Json<ItemData>,
) -> Result<impl Responder, MyError> {
    let name = &item_data.name;
    let description = &item_data.description;

    let client = db.lock().unwrap();

    client
        .execute(
            "INSERT INTO items (name, description) VALUES ($1, $2)",
            &[name, description],
        )
        .await
        .map_err(|e| MyError(format!("Database error: {}", e)))?;

    Ok(HttpResponse::Ok().body("Item created successfully"))
}

#[derive(Debug, Deserialize)]
struct UpdatedItemData {
    id: i32,
    name: String,
    description: String,
}

async fn update_item(
    db: web::Data<Arc<Mutex<Client>>>,
    updated_data: web::Json<UpdatedItemData>,
) -> Result<impl Responder, MyError> {
    let id = updated_data.id;
    let name = &updated_data.name;
    let description = &updated_data.description;

    let client = db.lock().unwrap();

    client
        .execute(
            "UPDATE items SET name = $1, description = $2 WHERE id = $3",
            &[name, description, &id],
        )
        .await
        .map_err(|e| MyError(format!("Database error: {}", e)))?;

    Ok(HttpResponse::Ok().body("Item updated successfully"))
}

async fn delete_item(
    db: web::Data<Arc<Mutex<Client>>>,
    item_id: web::Path<i32>,
) -> Result<impl Responder, MyError> {
    let id = item_id.into_inner();
    let client = db.lock().unwrap();
    
    client
        .execute("DELETE FROM items WHERE id = $1", &[&id])
        .await
        .map_err(|e| MyError(format!("Database error: {}", e)))?;

    Ok(HttpResponse::Ok().body("Item deleted successfully"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = Arc::new(Mutex::new(connect().await.unwrap()));
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db.clone()))
            .service(
                web::resource("/items")
                    .route(web::get().to(get_items))
                    .route(web::post().to(create_item))
                    .route(web::put().to(update_item))
                )
                .service(
                    web::resource("/items/{id}")
                        .route(web::delete().to(delete_item))
                )
        })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
