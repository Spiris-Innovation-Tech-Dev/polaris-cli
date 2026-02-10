use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    let specs = [
        ("auth_v2", "specs/auth-v2-oas3.yaml"),
        ("common", "specs/common-oas3.yaml"),
        ("issue_query_v1", "specs/issue-query-v1-oas3.yaml"),
        ("triage_command_v1", "specs/triage-command-v1-oas3.yaml"),
        ("triage_query_v1", "specs/triage-query-v1-oas3.yaml"),
    ];

    for (name, spec_path) in &specs {
        println!("cargo:rerun-if-changed={spec_path}");

        let spec_content = fs::read_to_string(spec_path)
            .unwrap_or_else(|e| panic!("Failed to read {spec_path}: {e}"));

        let spec: openapiv3::OpenAPI = serde_yaml::from_str(&spec_content)
            .unwrap_or_else(|e| panic!("Failed to parse {spec_path}: {e}"));

        let spec_clone = spec.clone();
        let result = std::panic::catch_unwind(move || {
            let mut g = progenitor::Generator::default();
            g.generate_tokens(&spec_clone)
        });

        let tokens = match result {
            Ok(Ok(t)) => t,
            Ok(Err(e)) => {
                eprintln!("Warning: skipping {spec_path}: {e}");
                let out_path = Path::new(&out_dir).join(format!("{name}.rs"));
                fs::write(&out_path, "// Generation failed — hand-craft this module\n").unwrap();
                continue;
            }
            Err(_) => {
                eprintln!("Warning: skipping {spec_path}: generator panicked");
                let out_path = Path::new(&out_dir).join(format!("{name}.rs"));
                fs::write(&out_path, "// Generation panicked — hand-craft this module\n").unwrap();
                continue;
            }
        };

        let content = format!("{tokens}");

        let formatted = if let Ok(f) = rustfmt_wrapper::rustfmt(content.clone()) {
            f
        } else {
            content
        };

        let out_path = Path::new(&out_dir).join(format!("{name}.rs"));
        fs::write(&out_path, formatted)
            .unwrap_or_else(|e| panic!("Failed to write {}: {e}", out_path.display()));

        eprintln!("Generated {name} successfully");
    }
}
