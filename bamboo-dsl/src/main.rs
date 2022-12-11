fn main() {
    let whatever = r#"
(addStep 
    :name "Add python" 
    :run "/sbin/apk add --update python3"
)"#;

    println!("{:?}", whatever);
    println!("{:?}", bamboo_dsl::parse_config(whatever.to_string()))
}
