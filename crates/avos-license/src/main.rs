use avos_license::verify_offline_key;

fn main() {
    let edition = verify_offline_key(b"");
    println!("// AVOS-LICENSE // Active edition: {:?}", edition);
}
