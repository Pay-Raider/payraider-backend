use payraider_backend::openapi::ApiDoc;
use std::fs::File;
use std::io::Write;
use utoipa::OpenApi;

fn main() {
    let json = ApiDoc::openapi()
        .to_pretty_json()
        .expect("Failed to serialize OpenAPI spec");

    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/openapi.json");

    let mut file = File::create(path).expect("Failed to create docs/openapi.json");
    file.write_all(json.as_bytes())
        .expect("Failed to write to docs/openapi.json");
    println!("Successfully generated OpenAPI spec at {}", path);
}
