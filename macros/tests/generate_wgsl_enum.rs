use indoc::indoc;
use macros::generate_wgsl_enum;

#[test]
fn generate_wgsl_enum() {
    #[allow(unused)]
    #[generate_wgsl_enum("test.wgsl")]
    #[derive(Debug)]
    enum MyEnum {
        First,
        Second,
        Third = 42,
        Fourth,
    }

    let _ = MyEnum::First;
    let file_content = std::fs::read_to_string("test.wgsl").expect("file should have been created");
    let expected_content = indoc! {"
        const First: u32 = 0;
        const Second: u32 = 1;
        const Third: u32 = 42;
        const Fourth: u32 = 43;\
    "};

    assert_eq!(file_content, expected_content);
}
