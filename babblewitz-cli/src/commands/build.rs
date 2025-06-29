use crate::core::executor::ImplementationExecutor;
use crate::core::implementation::{find_all_implementations, Implementation};
use anyhow::Result;
use std::path::Path;

pub fn build_implementation(impl_path: &Path) -> Result<()> {
    let implementation = Implementation::load_from_path(impl_path)?;
    ImplementationExecutor::build_implementation(&implementation)?;
    Ok(())
}

pub fn build_all_implementations() -> Result<()> {
    let implementations = find_all_implementations()?;

    let mut success_count = 0;
    let total_count = implementations.len();
    let mut failed_implementations = Vec::new();

    // Build each implementation
    for implementation in &implementations {
        println!("📦 Implementation: {}", implementation.name);

        match ImplementationExecutor::build_implementation(implementation) {
            Ok(_) => {
                success_count += 1;
            }
            Err(e) => {
                println!("  ❌ Build failed: {}", e);
                failed_implementations.push(implementation.name.clone());
            }
        }
    }

    println!("=== BUILD SUMMARY ===");
    println!("✅ Successful builds: {}/{}", success_count, total_count);

    if !failed_implementations.is_empty() {
        println!("❌ Failed builds:");
        for impl_name in &failed_implementations {
            println!("  - {}", impl_name);
        }
        return Err(anyhow::anyhow!(
            "{} impl(s) failed to build",
            failed_implementations.len()
        ));
    }

    println!("🎉 All impls built successfully!");
    Ok(())
}
