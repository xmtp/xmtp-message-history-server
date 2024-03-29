use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::Mutex;

use futures::StreamExt;
use actix_web::http::StatusCode;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use uuid::Uuid;

// Define a state to hold the mapping of UUIDs to file names.
struct AppState {
    file_map: Mutex<HashMap<Uuid, String>>,
}

async fn upload_file(mut payload: web::Payload, data: web::Data<AppState>) -> impl Responder {
    // Create a new UUID for the file.
    let file_id = Uuid::new_v4();
    let file_name = format!("uploads/{}", file_id);

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

    // Insert the file ID and name into the state map.
    data.file_map.lock().unwrap().insert(file_id, file_name);

    HttpResponse::Ok().body(file_id.to_string())
}

async fn get_file(path: web::Path<Uuid>, data: web::Data<AppState>) -> impl Responder {
    let file_map = data.file_map.lock().unwrap();

    if let Some(file_name) = file_map.get(&path.into_inner()) {
        HttpResponse::Ok()
            .content_type("application/octet-stream")
            .body(std::fs::read(file_name).unwrap())
    } else {
        HttpResponse::NotFound().finish()
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::fs::create_dir_all("uploads").unwrap();

    let app_state = web::Data::new(AppState {
        file_map: Mutex::new(HashMap::new()),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(web::resource("/upload").route(web::post().to(upload_file)))
            .service(web::resource("/files/{id}").route(web::get().to(get_file)))
    })
    .bind("0.0.0.0:5558")?
    .run()
    .await
}


#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{body::to_bytes, web, App, test};
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[actix_rt::test]
    async fn test_file_upload_and_retrieval() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    file_map: Mutex::new(HashMap::new()),
                }))
                .service(web::resource("/upload").route(web::post().to(upload_file)))
                .service(web::resource("/files/{id}").route(web::get().to(get_file)))
        ).await;

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Not much of a message history bundle yet.").unwrap();

        // Read the file's contents into a Vec<u8>
        let file_contents = std::fs::read(temp_file.path()).unwrap();
        let payload = web::Bytes::from(file_contents);

        // Create the request with the file's bytes as the payload
        let req = test::TestRequest::post()
            .uri("/upload")
            .set_payload(payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        // Extract the file ID from the response
        let body = to_bytes(resp.into_body()).await.unwrap();
        let file_id_str = std::str::from_utf8(&body).unwrap();
        let file_id = Uuid::parse_str(file_id_str).unwrap();

        // Attempt to retrieve the file using its ID
        let req = test::TestRequest::get()
            .uri(&format!("/files/{}", file_id))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
    }
}
