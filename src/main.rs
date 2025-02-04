use actix_cors::Cors;
use actix_web::http::StatusCode;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use futures::StreamExt;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use uuid::Uuid;

mod cleanup;

const UPLOAD_DIR: &str = "uploads";

async fn upload_file(_req: HttpRequest, mut payload: web::Payload) -> impl Responder {
    // Create a new UUID for the file.
    let file_id = Uuid::new_v4();
    let file_name: String;

    #[cfg(not(test))]
    {
        file_name = format!("{UPLOAD_DIR}/{file_id}");
    }

    #[cfg(test)]
    {
        use tempfile::NamedTempFile;
        file_name = NamedTempFile::new()
            .unwrap()
            .path()
            .to_string_lossy()
            .to_string();
    }

    // Try to create the file.
    let mut file = match File::create(&file_name) {
        Ok(file) => file,
        Err(_) => return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Write payload to file.
    while let Some(chunk) = payload.next().await {
        let data = chunk.unwrap();
        if file.write_all(&data).is_err() {
            return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    HttpResponse::Ok().body(file_id.to_string())
}

async fn get_file(_req: HttpRequest, path: web::Path<Uuid>) -> impl Responder {
    let file_id = path.to_string();
    let file_path: PathBuf = format!("uploads/{}", file_id).into();

    if file_path.exists() {
        // Read the file's content
        let mut file = match File::open(file_path) {
            Ok(f) => f,
            Err(_) => return HttpResponse::NotFound().body("File not found"),
        };
        let mut buffer = Vec::new();
        if file.read_to_end(&mut buffer).is_err() {
            return HttpResponse::InternalServerError().body("Failed to read file");
        }

        HttpResponse::Ok() // Serve the file content
            .content_type("application/octet-stream")
            .body(buffer)
    } else {
        HttpResponse::NotFound().finish()
    }
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;
    std::fs::create_dir_all("uploads")?;

    #[cfg(not(test))]
    cleanup::spawn_worker();

    let host = "0.0.0.0:5558";
    println!("Starting server at: {}", host);
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .route("/", web::get().to(health_check))
            .service(web::resource("/upload").route(web::post().to(upload_file)))
            .service(web::resource("/files/{id}").route(web::get().to(get_file)))
    })
    .bind(host)?
    .run()
    .await?;

    Ok(())
}

fn init_tracing() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::body;
    use actix_web::test;
    use std::path::Path;

    #[actix_web::test]
    async fn test_upload_file() {
        let app =
            test::init_service(App::new().route("/upload", web::post().to(upload_file))).await;

        let correct_payload = b"correct payload";

        let req = test::TestRequest::post()
            .uri("/upload")
            .set_payload(correct_payload.to_vec())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body = resp.into_body();
        let bytes = body::to_bytes(body).await.unwrap();
        assert!(Uuid::try_parse_ascii(&bytes).is_ok());
    }

    #[actix_web::test]
    async fn test_get_file() {
        let file_contents = b"this too shall pass!"; // Simulated file contents

        let uploads_path = "uploads";
        std::fs::create_dir_all(uploads_path).unwrap();

        let file_id = Uuid::new_v4();
        let file_name = Path::new(uploads_path).join(file_id.to_string());
        let mut file = File::create(&file_name).unwrap();
        let _ = file.write_all(file_contents);

        let app = test::init_service(
            App::new().service(web::resource("/files/{id}").route(web::get().to(get_file))),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/files/{file_id}"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "Should succeed");

        std::fs::remove_dir_all(uploads_path).expect("Failed to remove uploads directory");
    }
}
