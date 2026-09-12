fn main() {
    // The daemon's mount policy lives in this crate and in symbiote-sandbox,
    // so the proofs that drive the built binary check it against the content
    // this build consumed rather than against file timestamps.
    symbiote_source_stamp::build_stamp();
}
