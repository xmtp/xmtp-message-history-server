use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use actix_web::http::StatusCode;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use futures::StreamExt;
use hex::{decode, encode};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

async fn upload_file(_req: HttpRequest, mut payload: web::Payload) -> impl Responder {
    // Create a new UUID for the file.
    let file_id = Uuid::new_v4();
    let file_name: String;

    #[cfg(not(test))]
    {
        file_name = format!("uploads/{}", file_id)
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

async fn get_file(req: HttpRequest, path: web::Path<Uuid>) -> impl Responder {
    // Extract the X-HMAC header
    let hmac_header = match req.headers().get("X-HMAC") {
        Some(value) => value.to_str().unwrap_or_default(),
        None => return HttpResponse::BadRequest().body("Missing X-HMAC header"),
    };

    // Extract the X-SIGNING-KEY header
    let signing_key = match req.headers().get("X-SIGNING-KEY") {
        Some(value) => value.to_str().unwrap_or_default(),
        None => return HttpResponse::BadRequest().body("Missing X-SIGNING-KEY header"),
    };

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

        // Compute the HMAC for the file content
        let mut mac = HmacSha256::new_from_slice(signing_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(&buffer);
        let result_hmac = encode(mac.finalize().into_bytes());

        // Verify if the provided HMAC matches the computed one
        if hmac_header == result_hmac {
            HttpResponse::Ok() // Serve the file content
                .content_type("application/octet-stream")
                .body(buffer)
        } else {
            HttpResponse::Unauthorized().body("Invalid HMAC")
        }
    } else {
        HttpResponse::NotFound().finish()
    }
}

/// Verifies the HMAC of the request.
#[allow(dead_code)]
fn verify_hmac(
    hmac_header: &str,
    file_bytes: &[u8],
    signing_key: &[u8],
) -> Result<(), HttpResponse> {
    // Decode the hex HMAC
    let received_hmac = match decode(hmac_header) {
        Ok(hmac) => hmac,
        Err(_) => return Err(HttpResponse::BadRequest().body("Invalid HMAC format")),
    };

    // Create an instance of the HMAC-SHA256
    let mut mac = HmacSha256::new_from_slice(signing_key).expect("Insufficient HMAC key size");

    // Input the data to the HMAC instance
    mac.update(file_bytes);

    // Compute the HMAC and compare it with the received HMAC
    match mac.verify_slice(&received_hmac) {
        Ok(_) => Ok(()),
        Err(_) => Err(HttpResponse::Unauthorized().body("HMAC verification failed")),
    }
}


async fn health_check() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::fs::create_dir_all("uploads").unwrap();

    let host = "0.0.0.0:5558";
    println!("Starting server at: {}", host);
    HttpServer::new(move || {
        App::new()
            .route("/", web::get().to(health_check))
            .service(web::resource("/upload").route(web::post().to(upload_file)))
            .service(web::resource("/files/{id}").route(web::get().to(get_file)))
    })
    .bind(host)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::body;
    use actix_web::test;
    use std::path::Path;

    const SIGNING_KEY: &[u8] = b"TEST_SECRET_KEY";

    // Test helper function to create a HMAC signature
    fn create_hmac_signature(signing_key: &[u8], data: &[u8]) -> String {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(signing_key).expect("HMAC can take key of any size");
        mac.update(data);
        encode(mac.finalize().into_bytes())
    }

    // Tests the HMAC verification logic
    #[test]
    async fn test_hmac_verification() {
        // Test valid HMAC passes as Ok(())
        let correct_payload = b"correct payload";
        let correct_hmac = create_hmac_signature(SIGNING_KEY, correct_payload);
        let verify_correct = verify_hmac(&correct_hmac, correct_payload, SIGNING_KEY);

        assert!(verify_correct.is_ok(), "Should succeed with correct HMAC");

        // Test invvalid HMAC returns an HttpResponse())
        let incorrect_payload = b"incorrect payload";
        let verify_incorrect = verify_hmac(&correct_hmac, incorrect_payload, SIGNING_KEY);
        assert!(verify_incorrect.is_err());
    }

    #[actix_web::test]
    async fn test_upload_file() {
        let app =
            test::init_service(App::new().route("/upload", web::post().to(upload_file))).await;

        // Test with incorrect HMAC
        let req = test::TestRequest::get()
            .uri("/upload")
            .insert_header(("X-HMAC", "incorrect_hmac"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(
            resp.status().is_client_error(),
            "Should fail with incorrect HMAC"
        );

        let correct_payload = b"correct payload";
        let correct_hmac = create_hmac_signature(SIGNING_KEY, correct_payload);
        assert!(!correct_hmac.is_empty());

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
        let signing_key = std::str::from_utf8(SIGNING_KEY).unwrap().to_string();

        let file_contents = b"this too shall pass!"; // Simulated file contents
        let correct_hmac = create_hmac_signature(SIGNING_KEY, file_contents);
        let incorrect_hmac = String::from("none shall pass");

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

        // Test with correct HMAC
        let req = test::TestRequest::get()
            .uri(&format!("/files/{file_id}"))
            .insert_header(("X-HMAC", correct_hmac.clone()))
            .insert_header(("X-SIGNING-KEY", signing_key))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(
            resp.status().is_success(),
            "Should succeed with correct HMAC"
        );

        // Test with incorrect HMAC
        let req = test::TestRequest::get()
            .uri(&format!("/files/{file_id}"))
            .insert_header(("X-HMAC", incorrect_hmac))
            .insert_header(("X-SIGNING-KEY", "doesnt matter"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(
            resp.status().is_client_error(),
            "Should fail with incorrect HMAC"
        );

        // Test with incorrect Signing Key
        let req = test::TestRequest::get()
            .uri(&format!("/files/{file_id}"))
            .insert_header(("X-HMAC", correct_hmac))
            .insert_header(("X-SIGNING-KEY", "wrong"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(
            resp.status().is_client_error(),
            "Should fail with incorrect Signing Key"
        );
    }
}
