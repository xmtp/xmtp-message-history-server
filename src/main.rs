use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::sync::Mutex;

use actix_web::http::StatusCode;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use futures::StreamExt;
use hex::decode;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

// Define a state to hold the mapping of UUIDs to file names.
struct AppState {
    file_map: Mutex<HashMap<Uuid, String>>,
    secret_key: String,
}

async fn upload_file(
    req: HttpRequest,
    mut payload: web::Payload,
    data: web::Data<AppState>,
) -> impl Responder {
    let secret_key = data.secret_key.as_bytes();

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

    let mut buffer = Vec::new();
    let _ = file.read_to_end(&mut buffer);

    // Verify HMAC
    if let Err(response) = verify_hmac(&req, &buffer, secret_key) {
        return response;
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

/// Verifies the HMAC of the request.
fn verify_hmac(
    req: &HttpRequest,
    file_bytes: &[u8],
    secret_key: &[u8],
) -> Result<(), HttpResponse> {
    // Extract the HMAC from the header
    let hmac_header = match req.headers().get("X-HMAC") {
        Some(h) => h.to_str().unwrap_or_default(),
        None => return Err(HttpResponse::Unauthorized().body("Missing HMAC header")),
    };

    // Decode the hex HMAC
    let received_hmac = match decode(hmac_header) {
        Ok(hmac) => hmac,
        Err(_) => return Err(HttpResponse::BadRequest().body("Invalid HMAC format")),
    };

    // Create an instance of the HMAC-SHA256
    let mut mac = HmacSha256::new_from_slice(secret_key).expect("HMAC can take key of any size");

    // Input the data to the HMAC instance
    mac.update(file_bytes);

    // Compute the HMAC and compare it with the received HMAC
    match mac.verify_slice(&received_hmac) {
        Ok(_) => Ok(()),
        Err(_) => Err(HttpResponse::Unauthorized().body("HMAC verification failed")),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::fs::create_dir_all("uploads").unwrap();

    let app_state = web::Data::new(AppState {
        file_map: Mutex::new(HashMap::new()),
        secret_key: "super-long-super-secret-unique-key-goes-here".to_string(),
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
    use actix_web::dev::ServiceResponse;
    use actix_web::{body::to_bytes, test, web, App};
    use hex::encode;
    use sha2::Sha256;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[actix_rt::test]
    #[ignore]
    async fn test_file_upload_and_retrieval() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    file_map: Mutex::new(HashMap::new()),
                    secret_key: "test-test-super-long-super-secret-unique-key-goes-here"
                        .to_string(),
                }))
                .service(web::resource("/upload").route(web::post().to(upload_file)))
                .service(web::resource("/files/{id}").route(web::get().to(get_file))),
        )
        .await;

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

    // Helper function to create a HMAC signature
    fn create_hmac_signature(secret_key: &[u8], data: &[u8]) -> String {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(secret_key).expect("HMAC can take key of any size");
        mac.update(data);
        encode(mac.finalize().into_bytes())
    }

    // Tests the HMAC verification logic
    #[actix_rt::test]
    async fn test_hmac_verification() {
        let secret_key = b"your-secret-key";
        let app = test::init_service(App::new().route(
            "/test",
            web::post().to(|req: HttpRequest, body: web::Bytes| async {
                // Attempt to verify the HMAC
                match verify_hmac(&req, &body, &secret_key) {
                    Ok(_) => HttpResponse::Ok().finish(),
                    Err(err) => err,
                }
            }),
        ))
        .await;

        let correct_payload = b"correct payload";
        let incorrect_payload = b"incorrect payload";

        // Create a correct HMAC for the correct payload
        let correct_hmac = create_hmac_signature(secret_key, correct_payload);

        // Simulate sending a request with the correct HMAC
        let req = test::TestRequest::post()
            .uri("/test")
            .insert_header(("X-HMAC", correct_hmac))
            .set_payload(correct_payload.to_vec())
            .to_request();

        let resp: ServiceResponse = test::call_service(&app, req).await;
        assert!(
            resp.status().is_success(),
            "Should succeed with correct HMAC"
        );

        // Create an incorrect HMAC for the purpose of testing
        let incorrect_hmac = create_hmac_signature(secret_key, incorrect_payload);

        // Simulate sending a request with the incorrect HMAC
        let req = test::TestRequest::post()
            .uri("/test")
            .insert_header(("X-HMAC", incorrect_hmac))
            .set_payload(correct_payload.to_vec())
            .to_request();

        let resp: ServiceResponse = test::call_service(&app, req).await;
        assert!(
            resp.status().is_client_error(),
            "Should fail with incorrect HMAC"
        );
    }
}
