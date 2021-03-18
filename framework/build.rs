use protoc_rust::Customize;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

const PROTO_DIR: &str = "protos";

fn compile_protos(proto_dir: &Path, proto_out_dir: &Path) {
    let proto_files: Vec<_> = glob::glob(&proto_dir.join("*.proto").to_string_lossy())
        .unwrap()
        .filter_map(|path| path.ok())
        .collect();

    let path_slices: Vec<_> = proto_files
        .iter()
        .map(|path| path.to_str().unwrap())
        .collect();

    let protoc_args = protoc_rust::Args {
        out_dir: &proto_out_dir.to_string_lossy(),
        input: path_slices.as_slice(),
        includes: &[&proto_dir.to_string_lossy()],
        customize: Customize::default(),
    };

    protoc_rust::run(protoc_args).expect("Could not run protoc");
}

fn generate_mod_file(proto_out_dir: &Path) -> PathBuf {
    let rust_files: Vec<_> = glob::glob(&proto_out_dir.join("*.rs").to_string_lossy())
        .unwrap()
        .filter_map(|path| path.ok())
        .collect();

    let mod_path = proto_out_dir.join("mod.rs");
    let mut mod_writer = BufWriter::new(fs::File::create(&mod_path).unwrap());

    for rust_file in rust_files {
        let module_name = rust_file.file_stem().unwrap().to_string_lossy();
        let import_line = format!("pub mod {};\n", module_name);
        mod_writer.write_all(import_line.as_bytes()).unwrap();
    }

    mod_path
}

fn generate_proto_modules(out_dir: &Path) {
    let proto_dir = Path::new(PROTO_DIR);
    let proto_out_dir = Path::new(&out_dir).join("protos");
    let _ = fs::remove_dir_all(&proto_out_dir);
    fs::create_dir(&proto_out_dir).unwrap();

    compile_protos(&proto_dir, &proto_out_dir);
    let mod_path = generate_mod_file(&proto_out_dir);

    println!("cargo:rerun-if-changed={}", proto_dir.to_string_lossy());
    println!(
        "cargo:rustc-env=PROTO_MOD_PATH={}",
        mod_path.to_string_lossy()
    )
}

fn build_passes(out_dir: &Path) -> PathBuf {
    let passes_dir = Path::new("../llvm-passes").canonicalize().unwrap();
    let passes_install_dir = out_dir.join("passes");
    let passes_build_dir = passes_install_dir.join("build");
    let _ = fs::create_dir_all(&passes_build_dir);

    let mut config_child = Command::new("cmake")
        .arg(&passes_dir)
        .arg(format!(
            "-DCMAKE_INSTALL_PREFIX={}",
            passes_install_dir.to_string_lossy()
        ))
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg("-DCMAKE_CXX_COMPILER=clang++")
        .arg("-DCMAKE_C_COMPILER=clang")
        .current_dir(&passes_build_dir)
        .spawn()
        .expect("Could not run CMake");
    let success = config_child.wait().unwrap().success();
    if !success {
        panic!("Could not configure passes!");
    }

    let mut build_child = Command::new("cmake")
        .args(&["--build", "."])
        .args(&["--target", "install"])
        .current_dir(&passes_build_dir)
        .spawn()
        .expect("Could not build project");
    let success = build_child.wait().unwrap().success();
    if !success {
        panic!("Could not build passes!");
    }

    println!("cargo:rerun-if-changed={}", passes_dir.display());

    println!(
        "cargo:rustc-env=RTLIBS_INSTALL_DIR={}:{}",
        passes_install_dir.join("lib64").display(),
        passes_install_dir.join("lib").display()
    );

    passes_install_dir
}

fn build_objdump(out_dir: &Path, passes_install_dir: &Path) {
    let analysis_binaries_objdump_dir = out_dir.join("analysis_binaries_objdump");
    let bitcode_path = Path::new("tests/assets/objdump.bc");
    assert!(bitcode_path.exists());

    let mut wrapper_child = Command::new(passes_install_dir.join("bin/collab_fuzz_wrapper"))
        .arg(&analysis_binaries_objdump_dir)
        .arg(&bitcode_path)
        .args(&["--", "-ldl"])
        .spawn()
        .expect("Could not run compiler wrapper");
    let success = wrapper_child.wait().unwrap().success();
    if !success {
        panic!("Wrapper script failed for objdump!");
    }

    println!(
        "cargo:rustc-env=ANALYSIS_BINARIES_OBJDUMP_PATH={}",
        analysis_binaries_objdump_dir
            .canonicalize()
            .unwrap()
            .to_string_lossy()
    );

    println!("cargo:rerun-if-changed={}", bitcode_path.display());
}

fn build_cutoff(out_dir: &Path, passes_install_dir: &Path) {
    let analysis_binaries_cutoff_dir = out_dir.join("analysis_binaries_cutoff");
    let bitcode_path = Path::new("tests/assets/cutoff.bc");
    assert!(bitcode_path.exists());

    let mut wrapper_child = Command::new(passes_install_dir.join("bin/collab_fuzz_wrapper"))
        .arg(&analysis_binaries_cutoff_dir)
        .arg(&bitcode_path)
        .spawn()
        .expect("Could not run compiler wrapper");
    let success = wrapper_child.wait().unwrap().success();
    if !success {
        panic!("Wrapper script failed for cutoff!");
    }

    println!(
        "cargo:rustc-env=ANALYSIS_BINARIES_CUTOFF_PATH={}",
        analysis_binaries_cutoff_dir
            .canonicalize()
            .unwrap()
            .to_string_lossy()
    );

    println!("cargo:rerun-if-changed={}", bitcode_path.display());
}

fn build_count(out_dir: &Path, passes_install_dir: &Path) {
    let analysis_binaries_count_dir = out_dir.join("analysis_binaries_count");
    let bitcode_path = Path::new("tests/assets/count.bc");
    assert!(bitcode_path.exists());

    let mut wrapper_child = Command::new(passes_install_dir.join("bin/collab_fuzz_wrapper"))
        .arg(&analysis_binaries_count_dir)
        .arg(&bitcode_path)
        .spawn()
        .expect("Could not run compiler wrapper");
    let success = wrapper_child.wait().unwrap().success();
    if !success {
        panic!("Wrapper script failed for count!");
    }

    println!(
        "cargo:rustc-env=ANALYSIS_BINARIES_COUNT_PATH={}",
        analysis_binaries_count_dir
            .canonicalize()
            .unwrap()
            .to_string_lossy()
    );

    println!("cargo:rerun-if-changed={}", bitcode_path.display());
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    generate_proto_modules(&out_dir);

    // Needed by libSVM
    println!("cargo:rustc-link-lib=dylib=stdc++");

    let is_rls = out_dir.components().any(|comp| {
        if let Component::Normal(dir) = comp {
            if dir == OsStr::new("rls") {
                println!("Detected RLS build");
                true
            } else {
                false
            }
        } else {
            false
        }
    });

    if env::var("PROFILE").unwrap() != "release" && !is_rls {
        println!("Building passes and instrumented binaries...");

        let passes_install_dir = build_passes(&out_dir);
        build_objdump(&out_dir, &passes_install_dir);
        build_cutoff(&out_dir, &passes_install_dir);
        build_count(&out_dir, &passes_install_dir);
    } else if is_rls {
        let not_existent_path = "/not/existent/path";
        println!(
            "cargo:rustc-env=RTLIBS_INSTALL_DIR={}:{}",
            not_existent_path, not_existent_path
        );
        println!(
            "cargo:rustc-env=ANALYSIS_BINARIES_OBJDUMP_PATH={}",
            not_existent_path
        );
        println!(
            "cargo:rustc-env=ANALYSIS_BINARIES_CUTOFF_PATH={}",
            not_existent_path
        );
        println!(
            "cargo:rustc-env=ANALYSIS_BINARIES_COUNT_PATH={}",
            not_existent_path
        );
    }
}
