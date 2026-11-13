use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let proto_root = manifest_dir.join("../../proto");
    let envelope_proto = proto_root.join("keynear/v1/envelope.proto");

    println!("cargo:rerun-if-changed={}", envelope_proto.display());
    println!("cargo:rerun-if-changed={}", proto_root.display());

    prost_build::compile_protos(&[envelope_proto], &[proto_root])
        .expect("compile keynear proto3 schemas");
}
