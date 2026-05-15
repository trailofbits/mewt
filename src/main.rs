use std::sync::Arc;

use mewt::LanguageRegistry;
use mewt::run_main;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create language registry and register supported languages
    let mut registry = LanguageRegistry::new();
    registry.register(mewt::languages::cpp::engine::CppLanguageEngine::new());
    registry.register(mewt::languages::go::engine::GoLanguageEngine::new());
    registry.register(mewt::languages::javascript::engine::JavaScriptLanguageEngine::new());
    registry.register_resolver(
        mewt::languages::javascript::resolver::JavaScriptLanguageResolver::new(),
    );
    registry.register(mewt::languages::rust::engine::RustLanguageEngine::new());
    registry.register(mewt::languages::solidity::engine::SolidityLanguageEngine::new());
    registry.register(mewt::languages::r#move::engine::MoveLanguageEngine::new());
    registry.register_resolver(mewt::languages::r#move::resolver::MoveLanguageResolver::new());

    // Run the shared main function
    run_main(
        Arc::new(registry),
        "mewt",
        "Mutation testing framework",
        None,
    )
    .await?;
    Ok(())
}
