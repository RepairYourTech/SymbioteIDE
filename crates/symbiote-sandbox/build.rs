fn main() {
    // symbiote-sandbox-launch is the descriptor boundary every sandboxed
    // process is started through, so the proofs that drive it check the built
    // binary against the content this build consumed.
    symbiote_source_stamp::build_stamp();
}
