use std::{env, error::Error, time};

use git2::Repository;

fn get_current_commit_hash() -> Result<String, Box<dyn Error>> {
    let repo_dir = env::var("CARGO_MANIFEST_DIR")?;
    let repo = Repository::open(repo_dir)?;
    let hash = repo.head()?.peel_to_commit()?.id().to_string();
    Ok(hash)
}

fn main() {
    match get_current_commit_hash() {
        Ok(hash) => {
            println!("cargo::rustc-env=BUILD_COMMIT={}", hash);
        }
        _ => {}
    }
    println!("cargo::rustc-env=BUILD_TIME={}", time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap().as_secs());
}
