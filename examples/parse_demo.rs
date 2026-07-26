fn main() {
    let input = r#"

    a=1;b=2;foo(x, y, z=3)

    "#;
    let ast = fastplus::parse(input);
    if let Err(e) = ast {
        println!("{}", e);
    }
}
