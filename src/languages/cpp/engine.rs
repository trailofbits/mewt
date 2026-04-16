use std::sync::OnceLock;
use tree_sitter::{Language as TsLanguage, Node};

use crate::LanguageEngine;
use crate::mutations::COMMON_MUTATIONS;
use crate::patterns;
use crate::types::{Mutant, Mutation, PartialMutant, Target};
use crate::utils::{
    calculate_line_offset, is_in_comment, node_text, parse_source, visit_nodes_with_cursor,
};

use super::mutations::CPP_MUTATIONS;
use super::syntax::{fields, nodes};

static CPP_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();

unsafe extern "C" {
    fn tree_sitter_cpp() -> *const tree_sitter::ffi::TSLanguage;
}

pub struct CppLanguageEngine {
    mutations: Vec<Mutation>,
}

impl Default for CppLanguageEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CppLanguageEngine {
    pub fn new() -> Self {
        let mut mutations: Vec<Mutation> = Vec::new();
        mutations.extend_from_slice(COMMON_MUTATIONS);
        mutations.extend_from_slice(CPP_MUTATIONS);
        Self { mutations }
    }
}

impl LanguageEngine for CppLanguageEngine {
    fn name(&self) -> &'static str {
        "C++"
    }

    fn extensions(&self) -> &[&'static str] {
        &["cpp", "cc", "cxx", "hpp", "hxx"]
    }

    fn get_mutations(&self) -> &[Mutation] {
        &self.mutations
    }

    fn mutate(&self, target: &Target) -> Vec<Mutant> {
        let source = &target.text;

        let language =
            CPP_LANGUAGE.get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_cpp()) });

        let tree = match parse_source(source, language) {
            Some(t) => t,
            None => return Vec::new(),
        };
        let root = tree.root_node();

        let mut all_mutants = Vec::new();
        for m in &self.mutations {
            match m.slug {
                "ER" => {
                    all_mutants.extend(
                        patterns::replace(
                            root,
                            source,
                            &[
                                nodes::EXPRESSION_STATEMENT,
                                nodes::RETURN_STATEMENT,
                                nodes::DECLARATION,
                                nodes::IF_STATEMENT,
                                nodes::WHILE_STATEMENT,
                                nodes::FOR_STATEMENT,
                                nodes::FOR_RANGE_LOOP,
                                nodes::DO_STATEMENT,
                            ],
                            "throw std::runtime_error(\"mewt\");",
                            &|node, src| {
                                let text = node_text(node, src);
                                !text.contains("throw ")
                            },
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "ER")),
                    );
                }
                "CR" => {
                    all_mutants.extend(
                        patterns::wrap(
                            root,
                            source,
                            &[
                                nodes::EXPRESSION_STATEMENT,
                                nodes::RETURN_STATEMENT,
                                nodes::DECLARATION,
                                nodes::IF_STATEMENT,
                                nodes::WHILE_STATEMENT,
                                nodes::FOR_STATEMENT,
                                nodes::FOR_RANGE_LOOP,
                                nodes::DO_STATEMENT,
                            ],
                            "/* ",
                            " */",
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "CR")),
                    );
                }
                "IF" => all_mutants.extend(
                    patterns::replace_condition(
                        root,
                        source,
                        nodes::IF_STATEMENT,
                        fields::CONDITION,
                        &["if"],
                        "false",
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "IF")),
                ),
                "IT" => all_mutants.extend(
                    patterns::replace_condition(
                        root,
                        source,
                        nodes::IF_STATEMENT,
                        fields::CONDITION,
                        &["if"],
                        "true",
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "IT")),
                ),
                "WF" => {
                    all_mutants.extend(
                        patterns::replace_condition(
                            root,
                            source,
                            nodes::WHILE_STATEMENT,
                            fields::CONDITION,
                            &["while"],
                            "false",
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "WF")),
                    );
                    all_mutants.extend(
                        patterns::replace_condition(
                            root,
                            source,
                            nodes::DO_STATEMENT,
                            fields::CONDITION,
                            &["while"],
                            "false",
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "WF")),
                    );
                }
                "AS" => all_mutants.extend(
                    patterns::swap_args(root, source, &[nodes::CALL_EXPRESSION], fields::ARGUMENTS)
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "AS")),
                ),
                "AOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_EXPRESSION],
                        &["+", "-", "*", "/", "%"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "AOS")),
                ),
                "AAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::ASSIGNMENT_EXPRESSION],
                        &["+=", "-=", "*=", "/=", "%="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "AAOS")),
                ),
                "BOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_EXPRESSION],
                        &["&", "|", "^"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "BOS")),
                ),
                "BAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::ASSIGNMENT_EXPRESSION],
                        &["&=", "|=", "^="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "BAOS")),
                ),
                "BL" => all_mutants.extend(
                    patterns::shuffle_nodes(
                        root,
                        source,
                        &[nodes::BOOLEAN, nodes::BOOLEAN_FALSE],
                        &["true", "false"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "BL")),
                ),
                "COS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_EXPRESSION],
                        &["==", "!=", "<", "<=", ">", ">="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "COS")),
                ),
                "LOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_EXPRESSION],
                        &["&&", "||"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "LOS")),
                ),
                "SOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_EXPRESSION],
                        &["<<", ">>"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "SOS")),
                ),
                "SAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::ASSIGNMENT_EXPRESSION],
                        &["<<=", ">>="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "SAOS")),
                ),
                "LC" => all_mutants.extend(
                    patterns::shuffle_nodes(
                        root,
                        source,
                        &[nodes::BREAK_STATEMENT, nodes::CONTINUE_STATEMENT],
                        &["break;", "continue;"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "LC")),
                ),
                "NR" => all_mutants.extend(
                    patterns::remove_unary_operator(
                        root,
                        source,
                        nodes::UNARY_EXPRESSION,
                        fields::OPERATOR,
                        fields::ARGUMENT,
                        "!",
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "NR")),
                ),
                "DAS" => all_mutants.extend(
                    delete_array_swap_mutants(root, source)
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "DAS")),
                ),
                "MR" => all_mutants.extend(
                    move_removal_mutants(root, source)
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "MR")),
                ),
                "VR" => all_mutants.extend(
                    virtual_removal_mutants(root, source)
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "VR")),
                ),
                "RDV" => all_mutants.extend(
                    return_default_value_mutants(root, source)
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "RDV")),
                ),
                // Not applicable to C++
                "RZ" | "RCI" => {}
                _ => {
                    panic!(
                        "Unknown mutation slug encountered in C++ engine: {}",
                        m.slug
                    );
                }
            }
        }
        all_mutants
    }
}

/// DAS: Swap `delete ptr` ↔ `delete[] ptr` to detect scalar/array mismatch.
fn delete_array_swap_mutants(root: Node, source: &str) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        if node.kind() != "delete_expression" || is_in_comment(&node) {
            return;
        }

        let old_text = node_text(&node, source);
        let has_brackets = {
            let mut c = node.walk();
            node.children(&mut c).any(|child| child.kind() == "[")
        };

        // Find the operand — the first named child (the expression being deleted)
        let mut nc = node.walk();
        let operand = match node.named_children(&mut nc).next() {
            Some(op) => node_text(&op, source),
            None => return,
        };

        let new_text = if has_brackets {
            // delete[] x → delete x
            format!("delete {operand}")
        } else {
            // delete x → delete[] x
            format!("delete[] {operand}")
        };

        mutants.push(PartialMutant {
            byte_offset: node.start_byte() as u32,
            line_offset: calculate_line_offset(source, node.start_byte()),
            old_text: old_text.to_string(),
            new_text,
        });
    });
    mutants
}

/// MR: Remove std::move() wrapper, replacing `std::move(x)` with `x`.
fn move_removal_mutants(root: Node, source: &str) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        if node.kind() != "call_expression" || is_in_comment(&node) {
            return;
        }

        // Check if the callee is std::move or just move
        let mut nc = node.walk();
        let callee = match node.named_children(&mut nc).next() {
            Some(c) => c,
            None => return,
        };
        let callee_text = node_text(&callee, source);
        if callee_text != "std::move" && callee_text != "move" {
            return;
        }

        // Find the argument list and extract the single argument
        let mut nc2 = node.walk();
        let arg_list = match node
            .named_children(&mut nc2)
            .find(|c| c.kind() == "argument_list")
        {
            Some(al) => al,
            None => return,
        };

        let mut nc3 = arg_list.walk();
        let args: Vec<_> = arg_list.named_children(&mut nc3).collect();
        if args.len() != 1 {
            return;
        }

        let arg_text = node_text(&args[0], source);
        mutants.push(PartialMutant {
            byte_offset: node.start_byte() as u32,
            line_offset: calculate_line_offset(source, node.start_byte()),
            old_text: node_text(&node, source).to_string(),
            new_text: arg_text.to_string(),
        });
    });
    mutants
}

/// VR: Remove `virtual` keyword from method declarations/definitions.
/// Targets `field_declaration` and `function_definition` nodes that have
/// a `virtual` keyword as their first child.
fn virtual_removal_mutants(root: Node, source: &str) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        let kind = node.kind();
        if (kind != "field_declaration" && kind != "function_definition") || is_in_comment(&node) {
            return;
        }

        // Check if the first child is the `virtual` keyword
        let first_child = match node.child(0) {
            Some(c) => c,
            None => return,
        };
        if first_child.kind() != "virtual" {
            return;
        }

        let old_text = node_text(&node, source);
        // Remove "virtual " (keyword + trailing space) from the beginning
        let new_text = old_text
            .strip_prefix("virtual ")
            .unwrap_or(old_text)
            .to_string();

        if new_text != old_text {
            mutants.push(PartialMutant {
                byte_offset: node.start_byte() as u32,
                line_offset: calculate_line_offset(source, node.start_byte()),
                old_text: old_text.to_string(),
                new_text,
            });
        }
    });
    mutants
}

/// Walk up from a node to find its enclosing function_definition.
fn enclosing_function<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "function_definition" {
            return Some(parent);
        }
        current = parent.parent();
    }
    None
}

/// Determine the effective return type of a C++ function_definition.
/// Returns the type text (e.g., "int", "bool") and whether the declarator
/// is a pointer (e.g., `int* f()` → type="int", is_pointer=true).
fn cpp_return_type_info<'a>(func_node: &Node<'a>, source: &'a str) -> Option<(&'a str, bool)> {
    let type_node = func_node.child_by_field_name("type")?;
    let type_text = node_text(&type_node, source);

    // Check if the declarator is a pointer_declarator
    let declarator = func_node.child_by_field_name("declarator");
    let is_pointer = declarator
        .map(|d| d.kind() == "pointer_declarator")
        .unwrap_or(false);

    Some((type_text, is_pointer))
}

/// Map a C++ type to its default/zero value.
/// Uses keyword-based matching to handle multi-word types like
/// `unsigned int`, `long long`, `unsigned long long`, `long double`, etc.
fn cpp_type_default(type_text: &str, is_pointer: bool) -> Option<&'static str> {
    if is_pointer {
        return Some("nullptr");
    }
    let t = type_text.trim();

    // Exact matches first
    match t {
        "bool" => return Some("false"),
        "void" => return None,
        _ => {}
    }

    // Split into words for keyword matching to avoid false positives
    // (e.g., "Point" contains "int" but is not an integer type)
    let words: Vec<&str> = t.split_whitespace().collect();
    let has_word = |word: &str| words.iter().any(|w| *w == word);

    // Floating-point: "float", "double", "long double"
    if has_word("double") || has_word("float") {
        return Some("0.0");
    }

    // Integer types: multi-word types like "unsigned int", "long long", etc.
    let integer_keywords = [
        "int",
        "long",
        "short",
        "char",
        "unsigned",
        "signed",
        "size_t",
        "ssize_t",
        "int8_t",
        "int16_t",
        "int32_t",
        "int64_t",
        "uint8_t",
        "uint16_t",
        "uint32_t",
        "uint64_t",
        "ptrdiff_t",
        "intptr_t",
        "uintptr_t",
    ];
    if integer_keywords.iter().any(|kw| has_word(kw)) {
        return Some("0");
    }

    None
}

/// Generate RDV (Return Default Value) mutants for C++.
fn return_default_value_mutants(root: Node, source: &str) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        if node.kind() != "return_statement" || is_in_comment(&node) {
            return;
        }

        let func = match enclosing_function(&node) {
            Some(f) => f,
            None => return,
        };

        let (type_text, is_pointer) = match cpp_return_type_info(&func, source) {
            Some(info) => info,
            None => return,
        };

        let default = match cpp_type_default(type_text, is_pointer) {
            Some(d) => d,
            None => return,
        };

        // Find the returned expression — the first named child
        let mut nc = node.walk();
        let return_expr = match node.named_children(&mut nc).next() {
            Some(expr) => expr,
            None => return, // bare `return;`
        };

        let old_text = node_text(&return_expr, source);
        if old_text == default {
            return;
        }

        mutants.push(PartialMutant {
            byte_offset: return_expr.start_byte() as u32,
            line_offset: calculate_line_offset(source, return_expr.start_byte()),
            old_text: old_text.to_string(),
            new_text: default.to_string(),
        });
    });
    mutants
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashSet};
    use std::path::PathBuf;

    #[test]
    fn no_duplicate_slugs_in_combined_mutations() {
        let engine = CppLanguageEngine::new();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut dups: BTreeSet<String> = BTreeSet::new();
        for m in engine.get_mutations() {
            if !seen.insert(m.slug) {
                dups.insert(m.slug.to_string());
            }
        }
        assert!(
            dups.is_empty(),
            "Duplicate mutation slugs found in C++ engine: {dups:?}",
        );
    }

    #[test]
    fn all_defined_slugs_have_match_arms() {
        let text = "class B { virtual void f(); }; void g(int* p) { delete p; auto x = std::move(p); if (true) return; } int h() { return 42; }";
        let target = Target {
            id: 0,
            path: PathBuf::from("test.cpp"),
            file_hash: crate::types::Hash::digest(text.to_string()),
            text: text.to_string(),
            language: "C++".to_string(),
        };
        let engine = CppLanguageEngine::new();
        let _ = engine.mutate(&target);
    }
}
