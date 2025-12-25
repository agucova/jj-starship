//! Benchmark different approaches for computing change_id prefix length
//!
//! Run with: cargo run --release --bin prefix_benchmark

use jj_lib::config::{ConfigLayer, ConfigSource, StackedConfig};
use jj_lib::hex_util::encode_reverse_hex;
use jj_lib::id_prefix::IdPrefixContext;
use jj_lib::object_id::ObjectId;
use jj_lib::repo::{Repo, StoreFactories};
use jj_lib::revset::{RevsetExtensions, UserRevsetExpression};
use jj_lib::settings::UserSettings;
use jj_lib::workspace::{Workspace, default_working_copy_factories};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

fn create_user_settings() -> Result<UserSettings, Box<dyn std::error::Error>> {
    let mut config = StackedConfig::with_defaults();
    let mut user_layer = ConfigLayer::empty(ConfigSource::User);
    user_layer.set_value("user.name", "benchmark")?;
    user_layer.set_value("user.email", "benchmark@localhost")?;
    config.add_layer(user_layer);
    Ok(UserSettings::from_config(config)?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <jj-repo-path>", args[0]);
        eprintln!("Example: {} .", args[0]);
        std::process::exit(1);
    }

    let repo_root = PathBuf::from(&args[1]);
    if !repo_root.join(".jj").exists() {
        eprintln!("Error: {} is not a JJ repository", repo_root.display());
        std::process::exit(1);
    }

    println!("Benchmarking prefix computation approaches in: {}", repo_root.display());
    println!();

    let settings = create_user_settings()?;
    let workspace = Workspace::load(
        &settings,
        &repo_root,
        &StoreFactories::default(),
        &default_working_copy_factories(),
    )?;

    let repo: Arc<jj_lib::repo::ReadonlyRepo> = workspace.repo_loader().load_at_head()?;
    let view = repo.view();
    let wc_id = view
        .wc_commit_ids()
        .get(workspace.workspace_name())
        .ok_or("no working copy")?;
    let commit = repo.store().get_commit(wc_id)?;
    let change_id_full = encode_reverse_hex(commit.change_id().as_bytes());
    let change_id = &change_id_full[..8.min(change_id_full.len())];

    const ITERATIONS: usize = 10;

    // Approach 1: Fixed 4-character prefix (current solution)
    println!("=== Approach 1: Fixed 4-character prefix ===");
    let mut total = std::time::Duration::ZERO;
    for i in 0..ITERATIONS {
        let start = Instant::now();
        let prefix_len = 4.min(change_id.len());
        let elapsed = start.elapsed();
        total += elapsed;
        if i == 0 {
            println!("First run: {:?} (prefix_len = {})", elapsed, prefix_len);
        }
    }
    let avg = total / ITERATIONS as u32;
    println!("Average over {} iterations: {:?}", ITERATIONS, avg);
    println!();

    // Approach 2: Using repo.shortest_unique_change_id_prefix_len()
    println!("=== Approach 2: repo.shortest_unique_change_id_prefix_len() ===");
    let mut total = std::time::Duration::ZERO;
    for i in 0..ITERATIONS {
        let start = Instant::now();
        let prefix_len = repo
            .shortest_unique_change_id_prefix_len(commit.change_id())
            .unwrap_or(8)
            .min(change_id.len());
        let elapsed = start.elapsed();
        total += elapsed;
        if i == 0 {
            println!("First run: {:?} (prefix_len = {})", elapsed, prefix_len);
        }
    }
    let avg = total / ITERATIONS as u32;
    println!("Average over {} iterations: {:?}", ITERATIONS, avg);
    println!();

    // Approach 3: IdPrefixContext with limited scope (original approach)
    println!("=== Approach 3: IdPrefixContext with scoped disambiguation ===");
    let ancestor_depth = 10;
    let mut total = std::time::Duration::ZERO;
    for i in 0..ITERATIONS {
        let start = Instant::now();
        
        let extensions = Arc::new(RevsetExtensions::default());
        let wc_expr = UserRevsetExpression::working_copy(workspace.workspace_name().to_owned());
        let limited_expr = wc_expr.ancestors_range(0..(ancestor_depth as u64) + 1);
        let context = IdPrefixContext::new(extensions).disambiguate_within(limited_expr);
        
        let prefix_len = context
            .populate(repo.as_ref())
            .ok()
            .and_then(|idx| {
                idx.shortest_change_prefix_len(repo.as_ref(), commit.change_id())
                    .ok()
            })
            .unwrap_or(8)
            .min(change_id.len());
        
        let elapsed = start.elapsed();
        total += elapsed;
        if i == 0 {
            println!("First run: {:?} (prefix_len = {})", elapsed, prefix_len);
        }
    }
    let avg = total / ITERATIONS as u32;
    println!("Average over {} iterations: {:?}", ITERATIONS, avg);
    println!();

    println!("=== Summary ===");
    println!("For a prompt use case, approach 1 (fixed prefix) is recommended");
    println!("as it provides consistent sub-millisecond performance.");

    Ok(())
}
