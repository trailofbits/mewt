use std::sync::Arc;

use mewt::LanguageRegistry;
use mewt::languages as langs;
use mewt::run_main;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create language registry and register supported languages
    let mut registry = LanguageRegistry::new();
    registry.register_resolver(langs::cpp::resolver::CppLanguageResolver::new());
    registry.register_resolver(langs::daml::resolver::DamlLanguageResolver::new());
    registry.register_resolver(langs::go::resolver::GoLanguageResolver::new());
    registry.register_resolver(langs::javascript::resolver::JavaScriptLanguageResolver::new());
    registry.register_resolver(langs::rust::resolver::RustLanguageResolver::new());
    registry.register_resolver(langs::ruby::resolver::RubyLanguageResolver::new());
    registry.register_resolver(langs::solidity::resolver::SolidityLanguageResolver::new());
    registry.register_resolver(langs::r#move::resolver::MoveLanguageResolver::new());

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
