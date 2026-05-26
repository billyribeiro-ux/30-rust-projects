fn main() {
    let pw = std::env::args()
        .nth(1)
        .expect("usage: genbcrypt <password>");
    print!("{}", bcrypt::hash(&pw, 12).unwrap());
}
