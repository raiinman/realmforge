fn main() {
    // Use the vendored protoc binary so the builder stage needs no system
    // protobuf-compiler package (container builds, CI).
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("vendored protoc");
    prost_build::Config::new()
        .protoc_executable(protoc)
        .compile_protos(&["proto/bgs.proto"], &["proto/"])
        .unwrap();
}
