fn main() {
    let arg = std::env::args().nth(1).expect("name or path");
    let md = if arg.contains('/') || arg.ends_with(".md") {
        std::fs::read_to_string(&arg).unwrap()
    } else {
        std::fs::read_to_string(format!("tests/fixtures/{arg}.md")).unwrap()
    };
    let json = md2conf2md::md_to_adf_json(&md).unwrap();
    let val: serde_json::Value = serde_json::from_str(&json).unwrap();
    println!("{}", serde_json::to_string(&val).unwrap());
}
