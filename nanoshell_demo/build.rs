use nanoshell_build::{FlutterBuild, FlutterBuildOptions};

fn main() {
    let build = FlutterBuild::new(FlutterBuildOptions {
        target_file: None,
        local_engine: match FlutterBuild::build_mode().as_str() {
            // "debug" => Some("host_debug".into()),
            "debug" => Some("host_debug_unopt".into()),
            "release" => Some("host_release".into()),
            _ => None,
        },
        local_engine_src_path: None,
    });
    if let Err(error) = build.build() {
        println!("Build failed with error:\n{}", error);
        panic!();
    }
}
