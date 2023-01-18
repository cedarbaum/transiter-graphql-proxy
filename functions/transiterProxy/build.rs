extern crate prost_build;

fn main() {
    prost_build::Config::new()
        .type_attribute(".", "#[derive(serde::Serialize,serde::Deserialize)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .compile_protos(&["src/proto/transiter/public.proto"], &["src/proto"])
        .unwrap();
}
