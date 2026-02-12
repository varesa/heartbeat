use heartbeat_core::{DynamoStore, Monitor, Slug};

fn main() {
    // Placeholder -- real implementation in Phase 3
    println!("heartbeat-checker placeholder");

    // Prove imports work
    let slug = Slug::new("nightly-backup").expect("valid slug");
    println!("Validated slug: {slug}");

    // Suppress unused import warnings
    let _ = std::any::type_name::<Monitor>();
    let _ = std::any::type_name::<DynamoStore>();
}
