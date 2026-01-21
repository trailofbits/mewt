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
)

declare -A GRAMMAR_PATHS=(
  ["rust"]="" # repo root
  ["solidity"]="" # repo root
  ["go"]="" # repo root
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
git clone "$repo_url" "$TEMP_DIR"
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
