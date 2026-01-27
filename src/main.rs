use std::sync::Arc;

use mewt::LanguageRegistry;
use mewt::run_main;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create language registry and register supported languages
    let mut registry = LanguageRegistry::new();
    registry.register(mewt::languages::go::engine::GoLanguageEngine::new());
    registry.register(mewt::languages::javascript::engine::JavaScriptLanguageEngine::new());
    registry.register(mewt::languages::rust::engine::RustLanguageEngine::new());
    registry.register(mewt::languages::solidity::engine::SolidityLanguageEngine::new());

    // Run the shared main function
    run_main(Arc::new(registry)).await?;
    Ok(())
}
