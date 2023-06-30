use chrono::{DateTime, Utc};
use chrono_tz::Europe::Warsaw;
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
    let now: DateTime<Utc> = Utc::now();
    let warsaw_now = now.with_timezone(&Warsaw);
    let formatted_now = warsaw_now.format("%Y-%m-%d %H:%M:%S").to_string();

    // Try to get commit hash from GITHUB_SHA environment variable first, then fallback to git command
    let git_hash = env::var("GITHUB_SHA").unwrap_or_else(|_| {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap()
    });

    // Cut the SHA down to 7 characters.
    let git_sha_short = &git_hash[0..7];

    // Try to get branch name from GITHUB_HEAD_REF environment variable first, then fallback to git command
    let git_branch = env::var("GITHUB_HEAD_REF").unwrap_or_else(|_| {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    });

    let version = env!("CARGO_PKG_VERSION").to_string();

    let build_info = BuildInfo {
        timestamp: formatted_now,
        commit_hash: git_sha_short.to_string(),
        branch_name: git_branch,
        version,
    };

    let file = File::create("build_info.json").expect("Failed to create file");
    to_writer_pretty(file, &build_info).expect("Failed to write to file");
}
