#[path = "../tests/support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "../tests/support/real_shapes.rs"]
mod real_shapes;
fn main() {
    let dir = std::env::args().nth(1).unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    for (n, b) in real_shapes::fixtures() {
        std::fs::write(format!("{dir}/{n}.jpg"), b).unwrap();
    }
}
