use chrono::Utc;
use serde::Serialize;
use serde_json::to_writer_pretty;
use std::env;
use std::fs::File;
use std::process::Command;

#[derive(Serialize)]
struct BuildInfo {
    timestamp: String,
    version: String,
    commit_hash: String,
    branch_name: String,
}

fn main() {
    let timestamp = Utc::now();

    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .expect("Failed to execute git command");

    let commit_hash = String::from_utf8(output.stdout).unwrap().trim().to_string();

    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .expect("Failed to execute git command");

    let branch_name = String::from_utf8(output.stdout).unwrap().trim().to_string();

    let version = env::var("CARGO_PKG_VERSION").unwrap();

    let info = BuildInfo {
        timestamp: timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
        version,
        commit_hash,
        branch_name,
    };

    let file = File::create("build_info.json").expect("Failed to create file");
    to_writer_pretty(file, &info).expect("Failed to write to file");
}
