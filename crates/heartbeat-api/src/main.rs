use heartbeat_core::{DynamoStore, Monitor, Slug};

fn main() {
    // Placeholder -- real implementation in Phase 2
    println!("heartbeat-api placeholder");

    // Prove imports work
    let slug = Slug::new("test-service").expect("valid slug");
    println!("Validated slug: {slug}");

    // Suppress unused import warnings
    let _ = std::any::type_name::<Monitor>();
    let _ = std::any::type_name::<DynamoStore>();
}
