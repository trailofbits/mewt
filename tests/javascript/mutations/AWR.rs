use crate::javascript::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn awr_removes_await_from_async_function() {
    let source = r#"
async function load(fetcher) {
  const value = await fetcher();
  return await Promise.resolve(value);
}
"#;

    assert_only_slug_and_expected_new_texts(
        source,
        "test.js",
        "AWR",
        &["fetcher()", "Promise.resolve(value)"],
    );
}

#[test]
fn awr_is_available_in_tsx() {
    let source = r#"
async function Component() {
  const name: string = await getName();
  return <div>{name}</div>;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.tsx", "AWR", &["getName()"]);
}
