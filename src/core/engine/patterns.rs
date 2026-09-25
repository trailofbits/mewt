use crate::types::PartialMutant;
use crate::utils::{calculate_line_offset, is_in_comment, node_text, visit_nodes_with_cursor};
use tree_sitter::Node;

/// Wrap entire nodes of the provided kinds with arbitrary prefix/suffix around the old text
pub fn wrap(
    root: Node,
    source: &str,
    node_kinds: &[&str],
    prefix: &str,
    suffix: &str,
) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    let kinds: Vec<&str> = node_kinds.to_vec();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        if kinds.contains(&node.kind())
            && !is_in_comment(&node)
            && !has_ancestor_with_kind(&node, &kinds)
        {
            let old = node_text(&node, source);
            let replacement = format!("{prefix}{old}{suffix}");
            mutants.push(PartialMutant {
                byte_offset: node.start_byte() as u32,
                line_offset: calculate_line_offset(source, node.start_byte()),
                old_text: old.to_string(),
                new_text: replacement,
            });
        }
    });
    mutants
}

/// Replace entire nodes of the provided kinds with a fixed replacement text
/// controlled by a filter predicate
pub fn replace(
    root: Node,
    source: &str,
    node_kinds: &[&str],
    replacement_text: &str,
    should_replace: &dyn Fn(&Node, &str) -> bool,
) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    let kinds: Vec<&str> = node_kinds.to_vec();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        if kinds.contains(&node.kind())
            && !is_in_comment(&node)
            && !has_ancestor_with_kind(&node, &kinds)
            && should_replace(&node, source)
        {
            mutants.push(PartialMutant {
                byte_offset: node.start_byte() as u32,
                line_offset: calculate_line_offset(source, node.start_byte()),
                old_text: node_text(&node, source).to_string(),
                new_text: replacement_text.to_string(),
            });
        }
    });
    mutants
}

/// Replace a condition for nodes of a specific kind using field-first, positional-fallback
pub fn replace_condition(
    root: Node,
    source: &str,
    node_kind: &str,
    condition_field_name: &str,
    keyword_kinds: &[&str],
    replacement: &str,
) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        if node.kind() == node_kind && !is_in_comment(&node) {
            if let Some(field_node) = node.child_by_field_name(condition_field_name) {
                let old_text = node_text(&field_node, source);
                let trimmed_start = old_text.trim_start();
                let trimmed_end = old_text.trim_end();
                let needs_parens = trimmed_start.starts_with('(') && trimmed_end.ends_with(')');
                let new_text = if needs_parens {
                    format!("({replacement})")
                } else {
                    replacement.to_string()
                };
                mutants.push(PartialMutant {
                    byte_offset: field_node.start_byte() as u32,
                    line_offset: calculate_line_offset(source, field_node.start_byte()),
                    old_text: old_text.to_string(),
                    new_text,
                });
            } else if let Some(cond) = first_named_child_after_keyword(&node, keyword_kinds) {
                if cond.kind() != ";" && cond.kind() != "{" {
                    let old_text = node_text(&cond, source);
                    let trimmed_start = old_text.trim_start();
                    let trimmed_end = old_text.trim_end();
                    let needs_parens = trimmed_start.starts_with('(') && trimmed_end.ends_with(')');
                    let new_text = if needs_parens {
                        format!("({replacement})")
                    } else {
                        replacement.to_string()
                    };
                    mutants.push(PartialMutant {
                        byte_offset: cond.start_byte() as u32,
                        line_offset: calculate_line_offset(source, cond.start_byte()),
                        old_text: old_text.to_string(),
                        new_text,
                    });
                }
            }
        }
    });
    mutants
}

/// Replace the first argument for calls whose callee matches a predicate
pub fn replace_first_arg(
    root: Node,
    source: &str,
    call_node_kinds: &[&str],
    args_field_name: &str,
    alt_args_kinds: &[&str],
    callee_matches: &dyn Fn(&str) -> bool,
    replacement: &str,
) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    let call_kinds: Vec<&str> = call_node_kinds.to_vec();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        if call_kinds.contains(&node.kind()) && !is_in_comment(&node) {
            let callee_text = if let Some(callee_node) = node.child(0) {
                node_text(&callee_node, source)
            } else {
                return;
            };
            if !callee_matches(callee_text) {
                return;
            }
            let args_node_opt = node.child_by_field_name(args_field_name).or_else(|| {
                let mut c = node.walk();
                for child in node.children(&mut c) {
                    let k = child.kind();
                    if alt_args_kinds.contains(&k) || k == args_field_name {
                        return Some(child);
                    }
                }
                None
            });
            if let Some(args_node) = args_node_opt {
                let mut ac = args_node.walk();
                for child in args_node.children(&mut ac) {
                    let k = child.kind();
                    if is_punctuation_kind(k) {
                        continue;
                    }
                    mutants.push(PartialMutant {
                        byte_offset: child.start_byte() as u32,
                        line_offset: calculate_line_offset(source, child.start_byte()),
                        old_text: node_text(&child, source).to_string(),
                        new_text: replacement.to_string(),
                    });
                    break;
                }
            }
        }
    });
    mutants
}

/// Replace each node of a given kind with its first named child.
pub fn replace_with_first_named_child(
    root: Node,
    source: &str,
    node_kind: &str,
) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        if node.kind() != node_kind || is_in_comment(&node) {
            return;
        }

        let mut nc = node.walk();
        let Some(child) = node.children(&mut nc).find(|child| child.is_named()) else {
            return;
        };

        mutants.push(PartialMutant {
            byte_offset: node.start_byte() as u32,
            line_offset: calculate_line_offset(source, node.start_byte()),
            old_text: node_text(&node, source).to_string(),
            new_text: node_text(&child, source).to_string(),
        });
    });
    mutants
}

/// Remove entire nodes of the provided kinds.
pub fn remove_nodes(root: Node, source: &str, node_kinds: &[&str]) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    let kinds: Vec<&str> = node_kinds.to_vec();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        if kinds.contains(&node.kind()) && !is_in_comment(&node) {
            mutants.push(PartialMutant {
                byte_offset: node.start_byte() as u32,
                line_offset: calculate_line_offset(source, node.start_byte()),
                old_text: node_text(&node, source).to_string(),
                new_text: String::new(),
            });
        }
    });
    mutants
}

/// Shuffle operator tokens inside expressions of specified kinds by replacing any occurrence
/// of the provided operators with any other in the set (excluding identity)
pub fn shuffle_operators(
    root: Node,
    source: &str,
    expr_node_kinds: &[&str],
    operators: &[&str],
) -> Vec<PartialMutant> {
    shuffle_impl(root, source, expr_node_kinds, operators, false)
}

/// Shuffle by replacing entire node text with alternatives from the operators set
pub fn shuffle_nodes(
    root: Node,
    source: &str,
    node_kinds: &[&str],
    alternatives: &[&str],
) -> Vec<PartialMutant> {
    shuffle_impl(root, source, node_kinds, alternatives, true)
}

fn shuffle_impl(
    root: Node,
    source: &str,
    node_kinds: &[&str],
    options: &[&str],
    full_node_mode: bool,
) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    let kinds: Vec<&str> = node_kinds.to_vec();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        if kinds.contains(&node.kind()) && !is_in_comment(&node) {
            if full_node_mode {
                let node_text_str = node_text(&node, source);
                for replacement in options.iter().copied() {
                    let matches = if options.len() == 2 {
                        node_text_str.contains(options[0]) || node_text_str.contains(options[1])
                    } else {
                        options.contains(&node_text_str)
                    };
                    if matches && replacement != node_text_str {
                        let new_text = if options.len() == 2 {
                            if node_text_str.contains(options[0]) {
                                node_text_str.replace(options[0], replacement)
                            } else {
                                node_text_str.replace(options[1], replacement)
                            }
                        } else {
                            replacement.to_string()
                        };
                        if new_text != node_text_str {
                            mutants.push(PartialMutant {
                                byte_offset: node.start_byte() as u32,
                                line_offset: calculate_line_offset(source, node.start_byte()),
                                old_text: node_text_str.to_string(),
                                new_text,
                            });
                        }
                    }
                }
            } else {
                let mut nc = node.walk();
                for child in node.children(&mut nc) {
                    let token = node_text(&child, source);
                    if options.contains(&token) {
                        for replacement in options.iter().copied() {
                            if replacement != token {
                                mutants.push(PartialMutant {
                                    byte_offset: child.start_byte() as u32,
                                    line_offset: calculate_line_offset(source, child.start_byte()),
                                    old_text: token.to_string(),
                                    new_text: replacement.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    });
    mutants
}

/// Swap adjacent arguments inside a child field (e.g., "arguments") for specified node kinds
pub fn swap_args(
    root: Node,
    source: &str,
    node_kinds: &[&str],
    args_field_name: &str,
) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    let kinds: Vec<&str> = node_kinds.to_vec();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        if kinds.contains(&node.kind()) && !is_in_comment(&node) {
            let mut args: Vec<Node> = Vec::new();

            if let Some(args_node) = node.child_by_field_name(args_field_name) {
                let mut ac = args_node.walk();
                for child in args_node.children(&mut ac) {
                    let k = child.kind();
                    if k != "(" && k != ")" && k != "," {
                        args.push(child);
                    }
                }
            } else {
                let mut ac = node.walk();
                for child in node.children(&mut ac) {
                    if child.kind() == "call_argument" {
                        if let Some(expr) = child.children(&mut child.walk()).find(|c| c.is_named())
                        {
                            args.push(expr);
                        } else {
                            args.push(child);
                        }
                    }
                }
            }

            if args.len() >= 2 {
                for i in 0..args.len() - 1 {
                    let a = args[i];
                    let b = args[i + 1];
                    let start = a.start_byte();
                    let end = b.end_byte();
                    let a_text = node_text(&a, source);
                    let b_text = node_text(&b, source);
                    let full_text = &source[start..end];
                    let swapped = format!("{b_text}, {a_text}");
                    mutants.push(PartialMutant {
                        byte_offset: start as u32,
                        line_offset: calculate_line_offset(source, start),
                        old_text: full_text.to_string(),
                        new_text: swapped,
                    });
                }
            }
        }
    });
    mutants
}

pub fn swap_named_children(root: Node, source: &str, node_kinds: &[&str]) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    let kinds: Vec<&str> = node_kinds.to_vec();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        if kinds.contains(&node.kind()) && !is_in_comment(&node) {
            let mut args: Vec<Node> = Vec::new();
            let mut ac = node.walk();
            for child in node.children(&mut ac) {
                if child.is_named() {
                    args.push(child);
                }
            }

            if args.len() >= 2 {
                for i in 0..args.len() - 1 {
                    let a = args[i];
                    let b = args[i + 1];
                    let start = a.start_byte();
                    let end = b.end_byte();
                    let a_text = node_text(&a, source);
                    let b_text = node_text(&b, source);
                    let full_text = &source[start..end];
                    let swapped = format!("{b_text}, {a_text}");
                    mutants.push(PartialMutant {
                        byte_offset: start as u32,
                        line_offset: calculate_line_offset(source, start),
                        old_text: full_text.to_string(),
                        new_text: swapped,
                    });
                }
            }
        }
    });
    mutants
}

/// Remove a unary operator from expressions of the given kind where the operator matches.
/// Replaces the entire unary expression with its operand (e.g., `!x` -> `x`).
/// Uses field-based child access with positional fallback for grammars without field names.
pub fn remove_unary_operator(
    root: Node,
    source: &str,
    node_kind: &str,
    operator_field: &str,
    operand_field: &str,
    target_operator: &str,
) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        if node.kind() != node_kind || is_in_comment(&node) {
            return;
        }

        // Try field-based access first, fall back to positional for grammars without fields
        let (op_text, operand_text) = if let (Some(op_node), Some(arg_node)) = (
            node.child_by_field_name(operator_field),
            node.child_by_field_name(operand_field),
        ) {
            (node_text(&op_node, source), node_text(&arg_node, source))
        } else {
            // Positional fallback: child(0) is operator, first named child is operand
            let op_node = match node.child(0) {
                Some(c) => c,
                None => return,
            };
            let mut nc = node.walk();
            let operand = node.children(&mut nc).find(|c| c.is_named());
            match operand {
                Some(arg_node) => (node_text(&op_node, source), node_text(&arg_node, source)),
                None => return,
            }
        };

        if op_text == target_operator {
            mutants.push(PartialMutant {
                byte_offset: node.start_byte() as u32,
                line_offset: calculate_line_offset(source, node.start_byte()),
                old_text: node_text(&node, source).to_string(),
                new_text: operand_text.to_string(),
            });
        }
    });
    mutants
}

////////////////////////////////////////
// Internal helpers

fn is_punctuation_kind(kind: &str) -> bool {
    kind == "(" || kind == ")" || kind == ","
}

fn is_keyword_kind(kind: &str, keywords: &[&str]) -> bool {
    keywords.contains(&kind)
}

fn has_ancestor_with_kind(node: &Node, kinds: &[&str]) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if kinds.contains(&parent.kind()) {
            return true;
        }
        current = parent.parent();
    }
    false
}

fn first_named_child_after_keyword<'a>(node: &Node<'a>, keywords: &[&str]) -> Option<Node<'a>> {
    let mut c = node.walk();
    for child in node.children(&mut c) {
        if child.is_missing() || child.is_error() {
            continue;
        }
        let k = child.kind();
        if is_keyword_kind(k, keywords) || is_punctuation_kind(k) {
            continue;
        }
        if child.is_named() {
            return Some(child);
        }
    }
    None
}
