use crate::cpp::integration_tests::mutants_for_slug;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    mutants_for_slug(source, "VR")
}

#[test]
fn test_vr_virtual_method_declaration() {
    let source = r#"
class Base {
    virtual void process();
};
"#;
    let vr = slug_mutants(source);
    assert_eq!(vr.len(), 1, "Should generate 1 VR mutation: {vr:?}");
    assert!(
        vr[0].old_text.starts_with("virtual"),
        "VR old_text should start with virtual: {:?}",
        vr[0].old_text
    );
    assert!(
        !vr[0].new_text.contains("virtual"),
        "VR new_text should not contain virtual: {:?}",
        vr[0].new_text
    );
}

#[test]
fn test_vr_virtual_destructor() {
    let source = r#"
class Base {
    virtual ~Base() {}
};
"#;
    let vr = slug_mutants(source);
    assert_eq!(
        vr.len(),
        1,
        "Should generate VR for virtual destructor: {vr:?}"
    );
    assert!(
        vr[0].new_text.contains("~Base"),
        "VR should preserve destructor name: {:?}",
        vr[0].new_text
    );
}

#[test]
fn test_vr_multiple_virtual_methods() {
    let source = r#"
class Base {
    virtual void f();
    virtual int g();
    void h();
};
"#;
    let vr = slug_mutants(source);
    assert_eq!(
        vr.len(),
        2,
        "Should generate VR for each virtual method, not for non-virtual: {vr:?}"
    );
}

#[test]
fn test_vr_non_virtual_not_mutated() {
    let source = r#"
class Concrete {
    void f();
    int g();
};
"#;
    let vr = slug_mutants(source);
    assert!(
        vr.is_empty(),
        "VR should not generate mutations for non-virtual methods: {vr:?}"
    );
}

#[test]
fn test_vr_in_comment_ignored() {
    let source = r#"
// virtual void f();
/* virtual int g(); */
class C {};
"#;
    let vr = slug_mutants(source);
    assert!(
        vr.is_empty(),
        "VR should not mutate inside comments: {vr:?}"
    );
}

#[test]
fn test_vr_override_without_virtual() {
    let source = r#"
class Derived {
    void f() override;
};
"#;
    let vr = slug_mutants(source);
    assert!(
        vr.is_empty(),
        "VR should not fire on methods with override but no virtual keyword: {vr:?}"
    );
}

#[test]
fn test_vr_pure_virtual() {
    let source = r#"
class Abstract {
    virtual void process() = 0;
};
"#;
    let vr = slug_mutants(source);
    assert_eq!(
        vr.len(),
        1,
        "VR should fire on pure virtual methods too: {vr:?}"
    );
    assert!(
        vr[0].new_text.contains("= 0"),
        "VR should preserve = 0 after removing virtual: {:?}",
        vr[0].new_text
    );
}
