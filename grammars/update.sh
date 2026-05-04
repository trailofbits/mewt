#!/usr/bin/env bash
set -euo pipefail

# Update tree-sitter grammar for a specific language
# Usage: update-grammar.sh <language> [dry_run]
# Examples:
#   update-grammar.sh rust true   # Preview what would be updated
#   update-grammar.sh rust false  # Actually perform the update

language="${1:-}"
dry_run="${2:-false}"

# Language configuration mappings for mewt (Rust and Solidity)
# To add a new language, just add entries to these associative arrays
declare -A REPO_URLS=(
  ["rust"]="https://github.com/tree-sitter/tree-sitter-rust"
  ["solidity"]="https://github.com/JoranHonig/tree-sitter-solidity"
  ["go"]="https://github.com/tree-sitter/tree-sitter-go"
  ["javascript"]="https://github.com/tree-sitter/tree-sitter-javascript"
  ["typescript"]="https://github.com/tree-sitter/tree-sitter-typescript"
  ["tsx"]="https://github.com/tree-sitter/tree-sitter-typescript"
  ["cpp"]="https://github.com/tree-sitter/tree-sitter-cpp"
  ["move-sui"]="https://github.com/MystenLabs/sui"
  ["move-iota"]="https://github.com/iotaledger/iota"
)

declare -A GRAMMAR_PATHS=(
  ["rust"]="" # repo root
  ["solidity"]="" # repo root
  ["go"]="" # repo root
  ["javascript"]="" # repo root
  ["typescript"]="typescript" # grammar is in typescript/ subdirectory
  ["tsx"]="tsx" # grammar is in tsx/ subdirectory
  ["cpp"]="" # repo root
  ["move-sui"]="external-crates/move/tooling/tree-sitter" # grammar in Sui monorepo
  ["move-iota"]="external-crates/move/tooling/tree-sitter" # grammar in IOTA monorepo
)

# Languages that require sparse checkout (large monorepos)
# Maps language -> subdirectory path to sparse-checkout
declare -A SPARSE_PATHS=(
  ["move-sui"]="external-crates/move/tooling/tree-sitter"
  ["move-iota"]="external-crates/move/tooling/tree-sitter"
)

# Validate language argument
if [ -z "$language" ]; then
  echo "Error: Language argument is required"
  echo "Usage: $0 <language> [dry_run]"
  echo "Supported languages: ${!REPO_URLS[*]}"
  exit 1
fi

# Check if language is supported
if [[ ! -v REPO_URLS["$language"] ]]; then
  echo "Error: Language '$language' is not supported"
  echo "Supported languages: ${!REPO_URLS[*]}"
  echo ""
  echo "To add support for a new language, add entries to REPO_URLS and GRAMMAR_PATHS in this script"
  exit 1
fi

# Get configuration for the specified language
repo_url="${REPO_URLS[$language]}"
grammar_path="${GRAMMAR_PATHS[$language]}"

if [ "$dry_run" = "true" ]; then
  echo "DRY RUN: Would update $language grammar (no changes will be made)"
  echo "Repository: $repo_url"
  echo "Grammar path: $grammar_path"
else
  echo "Updating $language grammar..."
  echo "Repository: $repo_url"
  echo "Grammar path: $grammar_path"
fi

# Step 1: Backup current grammar (temporary, outside repo)
echo "Backing up current grammar..."
BACKUP_DIR="/tmp/${language}-src.backup.$(date +%Y%m%d_%H%M%S)"
if [ -d "grammars/$language/src" ]; then
  if [ "$dry_run" = "false" ]; then
    rm -rf "$BACKUP_DIR"
    mkdir -p "$BACKUP_DIR"
    cp -r "grammars/$language/src" "$BACKUP_DIR/"
    echo "Backup created (temporary): $BACKUP_DIR"
  else
    echo "Would create temporary backup: $BACKUP_DIR"
  fi
fi

# Step 2: Clone upstream grammar repository
echo "Cloning upstream grammar repository..."
TEMP_DIR="/tmp/$language-grammar-update"
rm -rf "$TEMP_DIR"
if [[ -v SPARSE_PATHS["$language"] && -n "${SPARSE_PATHS[$language]}" ]]; then
  echo "Using sparse checkout for monorepo (path: ${SPARSE_PATHS[$language]})..."
  git clone --depth=1 --filter=blob:none --sparse "$repo_url" "$TEMP_DIR"
  git -C "$TEMP_DIR" sparse-checkout set "${SPARSE_PATHS[$language]}"
else
  git clone "$repo_url" "$TEMP_DIR"
fi
# Capture the vendored commit (latest of default branch)
vendored_commit="$(git -C "$TEMP_DIR" rev-parse HEAD)"

# Step 3: Verify generated files exist
echo "Verifying generated files..."
if [ ! -f "$TEMP_DIR/$grammar_path/src/parser.c" ]; then
  echo "Error: parser.c not found in upstream repository"
  echo "Expected: $TEMP_DIR/$grammar_path/src/parser.c"
  rm -rf "$TEMP_DIR"
  exit 1
fi

if [ ! -d "$TEMP_DIR/$grammar_path/src/tree_sitter" ]; then
  echo "Error: tree_sitter headers not found in upstream repository"
  echo "Expected: $TEMP_DIR/$grammar_path/src/tree_sitter/"
  rm -rf "$TEMP_DIR"
  exit 1
fi

# Handle dry run - files verified, show what would happen
if [ "$dry_run" = "true" ]; then
  echo ""
  echo "DRY RUN - Files verified successfully!"
  echo "Vendored commit would be: $vendored_commit"
  echo ""
  echo "Would perform these actions:"
  echo "  1. Copy $TEMP_DIR/$grammar_path/src/* -> grammars/$language/src/"
  echo "  2. Copy $TEMP_DIR/$grammar_path/grammar.js -> grammars/$language/"
  echo "  3. Create grammars/$language/vendor.json with commit: $vendored_commit"
  echo "  4. Run: cargo check"
  echo "  5. Run: cargo test parser"
  echo ""
  echo "Dry run completed - no changes made to your workspace"
  echo "Run 'bash grammars/update.sh $language false' to perform the actual update"
  rm -rf "$TEMP_DIR"
  exit 0
fi

# Step 4: Copy new files
echo "Copying new grammar files..."
rm -rf "grammars/$language/src"
mkdir -p "grammars/$language/src"
cp -r "$TEMP_DIR/$grammar_path/src/"* "grammars/$language/src/"
cp "$TEMP_DIR/$grammar_path/grammar.js" "grammars/$language/"

# Record vendored metadata for traceability
cat > "grammars/$language/vendor.json" <<EOF
{
  "repo": "$repo_url",
  "path": "$grammar_path",
  "commit": "$vendored_commit",
  "updated": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

# Step 4.5: Handle monorepo shared code (if present)
if [ -d "$TEMP_DIR/common" ]; then
  echo "Detected monorepo structure - copying shared code..."
  rm -rf "grammars/$language/common"
  cp -r "$TEMP_DIR/common" "grammars/$language/"
  
  echo "Rewriting import paths for self-contained vendoring..."
  # Fix JavaScript require('../common/...) -> require('./common/...)
  if [ -f "grammars/$language/grammar.js" ]; then
    sed -i.bak "s|require('../common/|require('./common/|g" "grammars/$language/grammar.js"
    rm -f "grammars/$language/grammar.js.bak"
  fi
  
  # Fix C includes "../../common/... -> "../common/...
  find "grammars/$language/src" -name "*.c" -o -name "*.h" | while read -r file; do
    sed -i.bak 's|"../../common/|"../common/|g' "$file"
    rm -f "$file.bak"
  done
  
  echo "Self-contained vendoring complete (no shared state between grammars)"
fi

# Step 5: Clean up temporary directory
echo "Cleaning up..."
rm -rf "$TEMP_DIR"

# Step 6: Test compilation
if command -v cargo >/dev/null 2>&1; then
  echo "Testing compilation..."
  if ! cargo check; then
    echo "Error: Compilation failed after grammar update"
    echo "You may need to update node type mappings in src/parser/$language.rs"
    exit 1
  fi
else
  echo "cargo not found, skipping compilation test"
fi

# Step 7: Run parser tests
if command -v cargo >/dev/null 2>&1; then
  echo "Running parser tests..."
  if ! cargo test parser; then
    echo "Warning: Parser tests failed - you may need to update node type mappings"
    echo "Check src/parser/$language.rs NodeType::from() mappings"
    exit 1
  fi
else
  echo "cargo not found, skipping parser tests"
fi

echo "Grammar update completed successfully!"
echo "Consider running 'just check && just build' to verify everything works"
echo "Ready to commit the changes"
