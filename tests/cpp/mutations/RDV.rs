use crate::cpp::integration_tests::mutants_for_slug;
use mewt::types::Mutant;

fn rdv_mutants(source: &str) -> Vec<Mutant> {
    mutants_for_slug(source, "RDV")
}

#[test]
fn test_rdv_int_return() {
    let source = r#"
int compute() {
    return 42;
}
"#;
    let rdv = rdv_mutants(source);
    assert_eq!(rdv.len(), 1, "Should generate 1 RDV mutation: {rdv:?}");
    assert_eq!(rdv[0].old_text, "42");
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_bool_return() {
    let source = r#"
bool isValid() {
    return true;
}
"#;
    let rdv = rdv_mutants(source);
    assert!(
        rdv.iter()
            .any(|m| m.old_text == "true" && m.new_text == "false"),
        "RDV should replace true with false: {rdv:?}"
    );
}

#[test]
fn test_rdv_float_return() {
    let source = r#"
double getRate() {
    return 3.14;
}
"#;
    let rdv = rdv_mutants(source);
    assert_eq!(rdv.len(), 1, "Should generate 1 RDV mutation: {rdv:?}");
    assert_eq!(rdv[0].old_text, "3.14");
    assert_eq!(rdv[0].new_text, "0.0");
}

#[test]
fn test_rdv_pointer_return() {
    let source = r#"
int* findNode() {
    return ptr;
}
"#;
    let rdv = rdv_mutants(source);
    assert_eq!(rdv.len(), 1, "Should generate 1 RDV mutation: {rdv:?}");
    assert_eq!(rdv[0].old_text, "ptr");
    assert_eq!(rdv[0].new_text, "nullptr");
}

#[test]
fn test_rdv_multiple_returns() {
    let source = r#"
int abs_val(int x) {
    if (x < 0) {
        return -x;
    }
    return x;
}
"#;
    let rdv = rdv_mutants(source);
    assert_eq!(
        rdv.len(),
        2,
        "Should generate 1 RDV per return statement: {rdv:?}"
    );
    assert!(rdv.iter().all(|m| m.new_text == "0"));
}

#[test]
fn test_rdv_skips_void_return() {
    let source = r#"
void doNothing() {
    return;
}
"#;
    let rdv = rdv_mutants(source);
    assert!(rdv.is_empty(), "RDV should not mutate void return: {rdv:?}");
}

#[test]
fn test_rdv_skips_auto_return() {
    let source = r#"
auto deduce() {
    return 42;
}
"#;
    let rdv = rdv_mutants(source);
    assert!(
        rdv.is_empty(),
        "RDV should skip auto return type (can't determine default): {rdv:?}"
    );
}

#[test]
fn test_rdv_skips_already_default() {
    let source = r#"
int zero() {
    return 0;
}
"#;
    let rdv = rdv_mutants(source);
    assert!(
        rdv.is_empty(),
        "RDV should not mutate when return value is already default: {rdv:?}"
    );
}

#[test]
fn test_rdv_skips_class_return() {
    let source = r#"
struct Point { int x; int y; };
Point origin() {
    return Point{0, 0};
}
"#;
    let rdv = rdv_mutants(source);
    assert!(
        rdv.is_empty(),
        "RDV should skip user-defined type returns: {rdv:?}"
    );
}

#[test]
fn test_rdv_stdint_types() {
    let source = r#"
uint32_t get_id() {
    return 12345;
}
"#;
    let rdv = rdv_mutants(source);
    assert_eq!(rdv.len(), 1, "Should generate RDV for uint32_t: {rdv:?}");
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_size_t_return() {
    let source = r#"
size_t count() {
    return 42;
}
"#;
    let rdv = rdv_mutants(source);
    assert_eq!(rdv.len(), 1, "Should generate RDV for size_t: {rdv:?}");
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_unsigned_int() {
    let source = r#"
unsigned int get_count() {
    return 42;
}
"#;
    let rdv = rdv_mutants(source);
    assert_eq!(
        rdv.len(),
        1,
        "Should generate RDV for unsigned int: {rdv:?}"
    );
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_long_long() {
    let source = r#"
long long get_big() {
    return 999999;
}
"#;
    let rdv = rdv_mutants(source);
    assert_eq!(rdv.len(), 1, "Should generate RDV for long long: {rdv:?}");
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_unsigned_long_long() {
    let source = r#"
unsigned long long get_huge() {
    return 123456789;
}
"#;
    let rdv = rdv_mutants(source);
    assert_eq!(
        rdv.len(),
        1,
        "Should generate RDV for unsigned long long: {rdv:?}"
    );
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_long_double() {
    let source = r#"
long double get_precise() {
    return 3.14159265358979L;
}
"#;
    let rdv = rdv_mutants(source);
    assert_eq!(rdv.len(), 1, "Should generate RDV for long double: {rdv:?}");
    assert_eq!(rdv[0].new_text, "0.0");
}

#[test]
fn test_rdv_signed_char() {
    let source = r#"
signed char get_byte() {
    return 'a';
}
"#;
    let rdv = rdv_mutants(source);
    assert_eq!(rdv.len(), 1, "Should generate RDV for signed char: {rdv:?}");
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_skips_std_string() {
    let source = r#"
#include <string>
std::string getName() {
    return "hello";
}
"#;
    let rdv = rdv_mutants(source);
    assert!(
        rdv.is_empty(),
        "RDV should skip std::string (not a primitive type): {rdv:?}"
    );
}

#[test]
fn test_rdv_skips_custom_type_with_keyword_substring() {
    // "interval" contains "int" as substring, "uint_wrapper" contains "uint"
    // Word-boundary matching should prevent false positives
    let source = r#"
struct interval { int lo; int hi; };
interval make_interval() {
    return interval{0, 10};
}
"#;
    let rdv = rdv_mutants(source);
    assert!(
        rdv.is_empty(),
        "RDV should not match 'interval' (contains 'int' as substring): {rdv:?}"
    );
}

#[test]
fn test_rdv_const_int_return() {
    let source = r#"
const int get_constant() {
    return 42;
}
"#;
    let rdv = rdv_mutants(source);
    // "const int" — type node might be "int" with a separate const qualifier,
    // or "const int" as the full text. Either way, should produce RDV.
    assert_eq!(rdv.len(), 1, "Should generate RDV for const int: {rdv:?}");
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_reference_return() {
    // int& return type — the & is in the declarator, not the type.
    // We detect pointer_declarator for T*, but reference_declarator for T&
    // is different. Document behavior.
    let source = r#"
int global = 42;
int& get_ref() {
    return global;
}
"#;
    let rdv = rdv_mutants(source);
    // The type field is "int", so this should produce RDV with default "0"
    assert_eq!(
        rdv.len(),
        1,
        "Should generate RDV for int& (type is still int): {rdv:?}"
    );
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_skips_template_return() {
    let source = r#"
#include <vector>
std::vector<int> get_vec() {
    return {1, 2, 3};
}
"#;
    let rdv = rdv_mutants(source);
    assert!(
        rdv.is_empty(),
        "RDV should skip template return types: {rdv:?}"
    );
}

#[test]
fn test_rdv_char_return() {
    let source = r#"
char get_initial() {
    return 'A';
}
"#;
    let rdv = rdv_mutants(source);
    assert_eq!(rdv.len(), 1, "Should generate RDV for char: {rdv:?}");
    assert_eq!(rdv[0].new_text, "0");
}
