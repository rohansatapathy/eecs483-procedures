#[link(name = "compiled_code", kind = "static")]
extern "sysv64" {
    #[link_name = "\x01entry"]
    fn entry(param: i64) -> i64;
}

#[export_name = "\x01print"]
extern "sysv64" fn print(x: i64) -> i64 {
    println!("{}", x);
    x
}

#[export_name = "\x01big_fun_nine"]
extern "sysv64" fn big_fun_nine(
    x1: i64, x2: i64, x3: i64, x4: i64, x5: i64, x6: i64, x7: i64, x8: i64,
    x9: i64,
) -> i64 {
    println!(
        "x1: {}\nx2: {}\nx3: {}\nx4: {}\nx5: {}\nx6: {}\nx7: {}\nx8: {}\nx9: {}",
        x1, x2, x3, x4, x5, x6, x7, x8, x9
    );
    x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9
}

#[export_name = "\x01big_fun_ten"]
extern "sysv64" fn big_fun_ten(
    x1: i64, x2: i64, x3: i64, x4: i64, x5: i64, x6: i64, x7: i64, x8: i64,
    x9: i64, x10: i64,
) -> i64 {
    println!(
        "x1: {}\nx2: {}\nx3: {}\nx4: {}\nx5: {}\nx6: {}\nx7: {}\nx8: {}\nx9: {}\nx10: {}",
        x1, x2, x3, x4, x5, x6, x7, x8, x9, x10
    );
    x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9 + x10
}

fn main() {
    let arg = std::env::args()
        .nth(1)
        .expect("no argument provided")
        .parse::<i64>()
        .expect("invalid argument for i64");
    let output = unsafe { entry(arg) };
    println!("{}", output);
}
