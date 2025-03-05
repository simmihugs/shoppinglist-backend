use actix_web::{App, HttpResponse, HttpServer, Responder, web};
//use rusqlite::{Connection, Result as SqliteResult};
use env_logger;
use log::{error, info};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
struct ShoppingItem {
    id: Option<i32>,
    name: String,
    is_shopped: bool,
    order_index: i32,
}

struct AppState {
    db: Mutex<Connection>,
}

async fn get_shopping_list(data: web::Data<AppState>) -> impl Responder {
    let conn = match data.db.lock() {
        Ok(conn) => conn,
        Err(e) => {
            error!("Failed to acquire database lock: {:?}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut stmt = match conn.prepare(
        "SELECT id, name, is_shopped, order_index FROM shopping_items ORDER BY order_index",
    ) {
        Ok(stmt) => stmt,
        Err(e) => {
            error!("Failed to prepare SQL statement: {:?}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let items_result: Result<Vec<ShoppingItem>, rusqlite::Error> = stmt
        .query_map([], |row| {
            let is_shopped_str: String = row.get(2)?;

            let is_shopped = match is_shopped_str.to_lowercase().as_str() {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => false,
            };

            let order_index: i32 = row.get(3)?;

            Ok(ShoppingItem {
                id: row.get(0)?,
                name: row.get(1)?,
                is_shopped,
                order_index,
            })
        })
        .and_then(|iter| iter.collect());

    match items_result {
        Ok(items) => {
            info!("Successfully retrieved {} items", items.len());
            HttpResponse::Ok().json(items)
        }
        Err(e) => {
            error!("Failed to retrieve shopping items: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn add_item(item: web::Json<ShoppingItem>, data: web::Data<AppState>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let result = conn.execute(
        "INSERT INTO shopping_items (name, is_shopped, order_index) VALUES (?1, ?2, ?3)",
        &[
            &item.name,
            &item.is_shopped.to_string(),
            &item.order_index.to_string(),
        ],
    );

    match result {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn update_item_status(item_id: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let result = conn.execute(
        "UPDATE shopping_items SET is_shopped = NOT is_shopped WHERE id = ?1",
        [item_id.into_inner()],
    );

    match result {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn update_item_order(
    item: web::Json<ShoppingItem>,
    data: web::Data<AppState>,
) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let result = conn.execute(
        "UPDATE shopping_items SET order_index = ?1 WHERE id = ?2",
        &[&item.order_index, &item.id.unwrap()],
    );

    match result {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let conn = Connection::open("shopping_list.db").unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS shopping_items (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            is_shopped BOOLEAN NOT NULL,
            order_index INTEGER NOT NULL
        )",
        [],
    )
    .unwrap();

    let app_state = web::Data::new(AppState {
        db: Mutex::new(conn),
    });

    let host = "192.168.178.22";
    let port = 8080;

    println!("Server running at http://{}:{}", host, port);

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/items", web::get().to(get_shopping_list))
            .route("/items", web::post().to(add_item))
            .route("/items/{id}/toggle", web::put().to(update_item_status))
            .route("/items/reorder", web::put().to(update_item_order))
    })
    .bind((host, port))?
    .run()
    .await
}
