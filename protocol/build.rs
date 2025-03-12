// Copyright (c) Sean Lawlor
//
// This source code is licensed under both the MIT license found in the
// LICENSE-MIT file in the root directory of this source tree.

//! This is the pre-compilation build script for the crate `ractor` when running in distributed
//! mode. It's used to compile protobuf into Rust code prior to compilation.

/// The shared-path for all protobuf specifications
/// The list of protobuf files to generate inside PROBUF_BASE_DIRECTORY

fn build_client_protobufs() {
    let  PROTOBUF_BASE_DIRECTORY: &str = "src";
    let PROTOBUF_FILES: [&str; 3] = ["cmd", "login", "store"];
    let mut protobuf_files = Vec::with_capacity(PROTOBUF_FILES.len());

    for file in PROTOBUF_FILES.iter() {
        let proto_file = format!("{PROTOBUF_BASE_DIRECTORY}/client/{file}.proto");
        println!("cargo:rerun-if-changed={proto_file}");
        protobuf_files.push(proto_file);
    }
    prost_build::compile_protos(&protobuf_files, &[PROTOBUF_BASE_DIRECTORY]).unwrap();
}
fn build_cluster_protobufs() {
    let PROTOBUF_BASE_DIRECTORY: &str = "src";
    let  PROTOBUF_FILES: [&str; 1] = ["node"];
    let mut protobuf_files = Vec::with_capacity(PROTOBUF_FILES.len());

    for file in PROTOBUF_FILES.iter() {
        let proto_file = format!("{PROTOBUF_BASE_DIRECTORY}/cluster/{file}.proto");
        println!("cargo:rerun-if-changed={proto_file}");
        protobuf_files.push(proto_file);
    }
    prost_build::compile_protos(&protobuf_files, &[PROTOBUF_BASE_DIRECTORY]).unwrap();
}

fn main() {
    // compile the spec files into Rust code
    build_client_protobufs();
    build_cluster_protobufs();
}
