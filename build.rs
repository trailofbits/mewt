use std::path::PathBuf;

fn build_grammar(dir: &PathBuf, lib_name: &str) {
    let mut build = cc::Build::new();
    build.include(dir).file(dir.join("parser.c"));

    // Include external scanner if present (required by some grammars like Rust)
    let scanner_c = dir.join("scanner.c");
    if scanner_c.exists() {
        build.file(scanner_c.clone());
    }

    // Suppress the specific warning from vendored tree-sitter code
    if build.get_compiler().is_like_clang() || build.get_compiler().is_like_gnu() {
        build.flag("-Wno-unused-but-set-variable");
    }

    // Compile to object file and link directly
    build.compile(lib_name);

    // Link the static library explicitly
    let out_dir = std::env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-arg={out_dir}/lib{lib_name}.a");

    // Tell cargo to rerun if the parser/scanner source changes
    println!("cargo:rerun-if-changed={}", dir.join("parser.c").display());
    let scanner_c = dir.join("scanner.c");
    if scanner_c.exists() {
        println!("cargo:rerun-if-changed={}", scanner_c.display());
    }
}

fn main() {
    // Override target-related environment variables to align with Nix expectations
    // The issue is cc crate converts aarch64-apple-darwin -> arm64-apple-macosx
    // but Nix expects arm64-apple-darwin
    unsafe {
        if let Ok(target) = std::env::var("TARGET")
            && target == "aarch64-apple-darwin"
        {
            // Force cc crate to use the darwin naming that Nix expects
            std::env::set_var(
                "CC_aarch64_apple_darwin",
                std::env::var("CC").unwrap_or_else(|_| "clang".to_string()),
            );
            std::env::set_var("CFLAGS_aarch64_apple_darwin", "-target arm64-apple-darwin");
        }
    }

    // Build Solidity grammar
    let solidity_dir: PathBuf = ["grammars", "solidity", "src"].iter().collect();
    build_grammar(&solidity_dir, "tree-sitter-solidity");

    // Build Rust grammar
    let rust_dir: PathBuf = ["grammars", "rust", "src"].iter().collect();
    build_grammar(&rust_dir, "tree-sitter-rust");

    // Build Go grammar
    let go_dir: PathBuf = ["grammars", "go", "src"].iter().collect();
    build_grammar(&go_dir, "tree-sitter-go");

    // Build JavaScript grammar
    let javascript_dir: PathBuf = ["grammars", "javascript", "src"].iter().collect();
    build_grammar(&javascript_dir, "tree-sitter-javascript");

    // Build TypeScript grammar
    let typescript_dir: PathBuf = ["grammars", "typescript", "src"].iter().collect();
    build_grammar(&typescript_dir, "tree-sitter-typescript");

    // Build TSX grammar (TypeScript + JSX)
    let tsx_dir: PathBuf = ["grammars", "tsx", "src"].iter().collect();
    build_grammar(&tsx_dir, "tree-sitter-tsx");
}
