use protobuf_codegen::Codegen;

fn main() {
    Codegen::new()
        .pure()
        .out_dir("src/generated")
        .input("src/protos/messages.proto")
        .include("src/protos")
        .run_from_script();
}
