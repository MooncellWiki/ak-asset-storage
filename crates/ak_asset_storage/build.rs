use std::process::Command;

fn main() {
    let output = Command::new("pnpm")
        .args(["run", "build:ui"])
        .current_dir("../../")
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        panic!("Command executed with failing error code");
    }
}
