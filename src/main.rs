use actix_cors::Cors;
use actix_web::http::StatusCode;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use client_events::render_event_card;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;
use xmtp_archive::ArchiveImporter;
use xmtp_proto::xmtp::device_sync::backup_element::Element;
use xmtp_proto::xmtp::device_sync::BackupElement;

mod cleanup;
mod client_events;

const FILESIZE_LIMIT: usize = 15_000_000;
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
    let mut file = match File::create(&file_name).await {
        Ok(file) => file,
        Err(_) => return HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Write payload to file.
    let mut size = 0;
    while let Some(chunk) = payload.next().await {
        let data = chunk.unwrap();

        size += data.len();
        if size > FILESIZE_LIMIT {
            // File is too large. Delete it and return an error.
            drop(file);
            let _ = tokio::fs::remove_file(&file_name).await;
            return HttpResponse::new(StatusCode::BAD_REQUEST);
        }

        if file.write_all(&data).await.is_err() {
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
        let mut file = match File::open(file_path).await {
            Ok(f) => f,
            Err(_) => return HttpResponse::NotFound().body("File not found"),
        };
        let mut buffer = Vec::new();
        if file.read_to_end(&mut buffer).await.is_err() {
            return HttpResponse::InternalServerError().body("Failed to read file");
        }

        HttpResponse::Ok() // Serve the file content
            .content_type("application/octet-stream")
            .body(buffer)
    } else {
        HttpResponse::NotFound().finish()
    }
}

async fn key_form(_req: HttpRequest) -> impl Responder {
    let page = include_str!("client_events/index.html");
    HttpResponse::Ok().content_type("text/html").body(page)
}

async fn styles(_req: HttpRequest) -> impl Responder {
    let styles = include_str!("client_events/styles.css");
    HttpResponse::Ok().content_type("text/css").body(styles)
}

#[derive(Serialize, Deserialize)]
struct CheckKeyForm {
    key: String,
}

impl CheckKeyForm {
    fn decode(key: &str) -> Result<(String, Vec<u8>), HttpResponse> {
        let mut split = key.split(":");
        let Some(file_id) = split.nth(0) else {
            return Err(HttpResponse::NotAcceptable().finish());
        };
        let Some(key) = split.nth(0) else {
            return Err(HttpResponse::NotAcceptable().finish());
        };
        let Ok(key) = hex::decode(key) else {
            return Err(HttpResponse::InternalServerError().body("Failed to decode key"));
        };

        Ok((file_id.to_string(), key))
    }
}

#[derive(Serialize, Deserialize)]
struct Event {
    id: usize,
    content: String,
    start: i64,
    group: String,
    #[serde(rename = "className")]
    class_name: String,
}
#[derive(Serialize, Deserialize)]
struct Group {
    id: String,
    content: String,
}

async fn client_events(_req: HttpRequest, form_data: web::Query<CheckKeyForm>) -> impl Responder {
    let (file_id, key) = match CheckKeyForm::decode(&form_data.key) {
        Ok(v) => v,
        Err(r) => return r,
    };

    let file_path: PathBuf = format!("uploads/{file_id}").into();
    let mut importer = ArchiveImporter::from_file(&file_path, &key).await.unwrap();

    let mut events = vec![];
    let mut groups = vec![Group {
        content: "Global".to_string(),
        id: "Global".to_string(),
    }];

    let mut i = 0;
    while let Some(Ok(element)) = importer.next().await {
        let BackupElement {
            element: Some(element),
        } = element
        else {
            continue;
        };

        let event_save = match element {
            Element::Event(event) => event,
            Element::Group(group_save) => {
                let mut attributes = group_save
                    .mutable_metadata
                    .map(|m| m.attributes)
                    .unwrap_or_default();
                let name = attributes.remove("group_name").unwrap_or_default();
                let id = hex::encode(&group_save.id);

                groups.push(Group {
                    id: id.clone(),
                    content: format!("{id}<br />{name}"),
                });
                continue;
            }
            _ => continue,
        };

        let content = render_event_card(event_save.event, &event_save.details);

        let group = match event_save.group_id {
            Some(group_id) => hex::encode(group_id),
            _ => "Global".to_string(),
        };

        events.push(Event {
            id: i,
            start: event_save.created_at_ns / 1000000,
            content,
            group,
            class_name: format!("blech"),
        });
        i += 1;
    }

    let page = include_str!("client_events/show.html");
    let page = page.replace(r#"{ itemdata }"#, &serde_json::to_string(&events).unwrap());
    let page = page.replace(r#"{ groupdata }"#, &serde_json::to_string(&groups).unwrap());
    HttpResponse::Ok().content_type("text/html").body(page)
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
            .service(web::resource("/key").route(web::get().to(key_form)))
            .service(web::resource("/client-events").route(web::get().to(client_events)))
            .service(web::resource("/styles.css").route(web::get().to(styles)))
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
        init_tracing().unwrap();

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

        let too_large = vec![0u8; 20_000_000];
        let req = test::TestRequest::post()
            .uri("/upload")
            .set_payload(too_large)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_get_file() {
        let file_contents = b"this too shall pass!"; // Simulated file contents

        let uploads_path = "uploads";
        std::fs::create_dir_all(uploads_path).unwrap();

        let file_id = Uuid::new_v4();
        let file_name = Path::new(uploads_path).join(file_id.to_string());
        let mut file = File::create(&file_name).await.unwrap();
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
