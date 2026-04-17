use std::path::Path;
use std::process::Command;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let schema_path = Path::new(&manifest_dir).join("schema.graphql");

    println!("cargo::rerun-if-changed=schema.graphql");
    println!("cargo::rerun-if-changed=Cargo.toml");
    println!("cargo::rerun-if-changed=assets/skills.md.tmpl");

    generate_skills_md(&manifest_dir);

    if schema_path.exists() {
        return;
    }

    let manifest_str = std::fs::read_to_string(Path::new(&manifest_dir).join("Cargo.toml"))
        .expect("Failed to read Cargo.toml");
    let manifest: toml::Value =
        toml::from_str(&manifest_str).expect("Failed to parse Cargo.toml");
    let commit = manifest["package"]["metadata"]["schema"]["commit"]
        .as_str()
        .expect("Missing [package.metadata.schema].commit in Cargo.toml");

    eprintln!("schema.graphql not found, downloading (commit: {commit})...");

    let endpoint = format!(
        "/repos/octokit/graphql-schema/contents/schema.graphql?ref={commit}"
    );
    let output = Command::new("gh")
        .args([
            "api",
            &endpoint,
            "-H",
            "Accept: application/vnd.github.raw+json",
        ])
        .output()
        .expect("Failed to execute `gh` CLI. Make sure `gh` is installed and authenticated.");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "Failed to download schema.graphql: {stderr}\n\
             You can manually download it:\n  \
             curl -L -o schema.graphql \
             https://raw.githubusercontent.com/octokit/graphql-schema/{commit}/schema.graphql"
        );
    }

    std::fs::write(&schema_path, &output.stdout)
        .unwrap_or_else(|e| panic!("Failed to write schema.graphql: {e}"));

    eprintln!(
        "Downloaded schema.graphql ({} bytes)",
        output.stdout.len()
    );
}

fn generate_skills_md(manifest_dir: &str) {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let version = std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION not set");

    let tmpl_path = Path::new(manifest_dir).join("assets/skills.md.tmpl");
    let tmpl = std::fs::read_to_string(&tmpl_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", tmpl_path.display()));

    let rendered = tmpl.replace("{{VERSION}}", &version);

    let dest = Path::new(&out_dir).join("skills.md");
    std::fs::write(&dest, rendered)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", dest.display()));
}
