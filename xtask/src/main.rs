// Copyright 2026 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;

use clap::Parser;
use clap::Subcommand;

fn workspace_dir() -> &'static Path {
    Path::new(env!("CARGO_WORKSPACE_DIR"))
}

#[derive(Parser)]
#[clap(about = "Run repository tasks.")]
struct Command {
    #[clap(subcommand)]
    sub: SubCommand,
}

impl Command {
    fn run(self) {
        match self.sub {
            SubCommand::Build(cmd) => cmd.run(),
            SubCommand::Lint(cmd) => cmd.run(),
            SubCommand::Package(cmd) => cmd.run(),
            SubCommand::Test(cmd) => cmd.run(),
        }
    }
}

#[derive(Subcommand)]
enum SubCommand {
    #[clap(about = "Compile all workspace targets.")]
    Build(CommandBuild),
    #[clap(about = "Run workspace quality checks.")]
    Lint(CommandLint),
    #[clap(about = "Build the publishable crate archives.")]
    Package(CommandPackage),
    #[clap(about = "Run workspace unit tests.")]
    Test(CommandTest),
}

#[derive(Parser)]
struct CommandBuild {
    #[arg(long, help = "Assert that `Cargo.lock` will remain unchanged.")]
    locked: bool,
}

impl CommandBuild {
    fn run(self) {
        run_command(make_build_cmd(self.locked));
    }
}

#[derive(Parser)]
struct CommandPackage {
    #[arg(long, help = "Assert that `Cargo.lock` will remain unchanged.")]
    locked: bool,
}

impl CommandPackage {
    fn run(self) {
        let main_version = package_version("serde-shape");
        let derive_version = package_version("serde-shape-derive");

        run_command(make_package_cmd("serde-shape-derive", self.locked));

        // Cargo would otherwise verify main against an already-published derive crate with the
        // same version. Unpack the archive and patch the two packaged crates together instead.
        run_command(make_package_archive_cmd("serde-shape", self.locked));
        unpack_package_archive("serde-shape", &main_version);
        run_command(make_verify_main_package_cmd(&main_version, &derive_version));
    }
}

#[derive(Parser)]
struct CommandTest {
    #[arg(long, help = "Run tests serially and do not capture output.")]
    no_capture: bool,
}

impl CommandTest {
    fn run(self) {
        run_command(make_test_cmd(self.no_capture, "serde-shape", &[]));
        run_command(make_test_cmd(self.no_capture, "serde-shape", &["std"]));
        run_command(make_test_cmd(self.no_capture, "serde-shape", &["derive"]));
        run_command(make_test_cmd(
            self.no_capture,
            "serde-shape",
            &["std", "derive"],
        ));
        run_command(make_test_cmd(
            self.no_capture,
            "serde-shape-test-no-std",
            &[],
        ));
        run_command(make_test_cmd(
            self.no_capture,
            "serde-shape-test-derive",
            &[],
        ));
        run_command(make_test_cmd(
            self.no_capture,
            "serde-shape-test-integration",
            &[],
        ));
    }
}

#[derive(Parser)]
#[clap(name = "lint")]
struct CommandLint {
    #[arg(long, help = "Automatically apply available lint and format fixes.")]
    fix: bool,
}

impl CommandLint {
    fn run(self) {
        run_command(make_clippy_cmd(self.fix));
        run_command(make_doc_cmd());
        run_command(make_format_cmd(self.fix));
        run_command(make_taplo_cmd(self.fix));
        run_command(make_typos_cmd());
        run_command(make_hawkeye_cmd(self.fix));
    }
}

fn find_command(cmd: &str) -> StdCommand {
    match which::which(cmd) {
        Ok(exe) => {
            let mut cmd = StdCommand::new(exe);
            cmd.current_dir(workspace_dir());
            cmd
        }
        Err(err) => {
            panic!("{cmd} not found: {err}");
        }
    }
}

fn ensure_installed(bin: &str, crate_name: &str) {
    if which::which(bin).is_err() {
        let mut cmd = find_command("cargo");
        cmd.args(["install", crate_name]);
        run_command(cmd);
    }
}

fn run_command(mut cmd: StdCommand) {
    println!("{cmd:?}");
    let status = cmd.status().expect("failed to execute process");
    assert!(status.success(), "command failed: {status}");
}

fn make_build_cmd(locked: bool) -> StdCommand {
    let mut cmd = find_command("cargo");
    cmd.args([
        "build",
        "--workspace",
        "--all-features",
        "--tests",
        "--examples",
        "--benches",
        "--bins",
    ]);
    if locked {
        cmd.arg("--locked");
    }
    cmd
}

fn make_package_cmd(package: &str, locked: bool) -> StdCommand {
    let mut cmd = find_command("cargo");
    cmd.args(["package", "--package", package, "--all-features"]);
    if locked {
        cmd.arg("--locked");
    }
    cmd
}

fn make_package_archive_cmd(package: &str, locked: bool) -> StdCommand {
    let mut cmd = make_package_cmd(package, locked);
    cmd.arg("--no-verify");
    cmd
}

fn unpack_package_archive(package: &str, version: &str) {
    let package_root = workspace_dir().join("target/package");
    let package_dir = package_root.join(format!("{package}-{version}"));
    if package_dir.exists() {
        fs::remove_dir_all(&package_dir).expect("failed to remove stale package directory");
    }

    let mut cmd = find_command("tar");
    cmd.arg("-xzf")
        .arg(package_root.join(format!("{package}-{version}.crate")))
        .arg("-C")
        .arg(package_root);
    run_command(cmd);
}

fn make_verify_main_package_cmd(main_version: &str, derive_version: &str) -> StdCommand {
    let package_dir = workspace_dir()
        .join("target/package")
        .join(format!("serde-shape-{main_version}"));
    let derive_dir = workspace_dir()
        .join("target/package")
        .join(format!("serde-shape-derive-{derive_version}"));
    let derive_dir = serde_json::to_string(&derive_dir.to_string_lossy()).unwrap();
    let patch = format!("patch.crates-io.serde-shape-derive.path={derive_dir}");

    let mut cmd = find_command("cargo");
    cmd.args(["check", "--manifest-path"])
        .arg(package_dir.join("Cargo.toml"))
        .args(["--all-features", "--config", &patch]);
    cmd
}

fn package_version(package: &str) -> String {
    let mut cmd = find_command("cargo");
    cmd.args(["metadata", "--no-deps", "--format-version", "1"]);
    let output = cmd.output().expect("failed to read workspace metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata should be valid JSON");
    metadata["packages"]
        .as_array()
        .and_then(|packages| {
            packages
                .iter()
                .find(|candidate| candidate["name"] == package)
        })
        .and_then(|package| package["version"].as_str())
        .unwrap_or_else(|| panic!("package {package} not found in cargo metadata"))
        .to_owned()
}

fn make_test_cmd(no_capture: bool, package: &str, features: &[&str]) -> StdCommand {
    let mut cmd = find_command("cargo");
    cmd.args(["test", "-p", package, "--no-default-features"]);
    if !features.is_empty() {
        cmd.args(["--features", features.join(",").as_str()]);
    }
    if no_capture {
        cmd.args(["--", "--nocapture"]);
    }
    cmd
}

fn make_format_cmd(fix: bool) -> StdCommand {
    let mut cmd = find_command("cargo");
    cmd.args(["+nightly", "fmt", "--all"]);
    if !fix {
        cmd.arg("--check");
    }
    cmd
}

fn make_clippy_cmd(fix: bool) -> StdCommand {
    let mut cmd = find_command("cargo");
    cmd.args([
        "+nightly",
        "clippy",
        "--tests",
        "--all-features",
        "--all-targets",
        "--workspace",
    ]);
    if fix {
        cmd.args(["--allow-staged", "--allow-dirty", "--fix"]);
    } else {
        cmd.args(["--", "-D", "warnings"]);
    }
    cmd
}

fn make_doc_cmd() -> StdCommand {
    let mut cmd = find_command("cargo");
    cmd.env("RUSTDOCFLAGS", "-D warnings --cfg docsrs");
    cmd.args([
        "+nightly",
        "doc",
        "--workspace",
        "--all-features",
        "--no-deps",
    ]);
    cmd
}

fn make_hawkeye_cmd(fix: bool) -> StdCommand {
    ensure_installed("hawkeye", "hawkeye");
    let mut cmd = find_command("hawkeye");
    if fix {
        cmd.args(["format"]);
    } else {
        cmd.args(["check"]);
    }
    cmd
}

fn make_typos_cmd() -> StdCommand {
    ensure_installed("typos", "typos-cli");
    find_command("typos")
}

fn make_taplo_cmd(fix: bool) -> StdCommand {
    ensure_installed("taplo", "taplo-cli");
    let mut cmd = find_command("taplo");
    if fix {
        cmd.args(["format"]);
    } else {
        cmd.args(["format", "--check"]);
    }
    cmd
}

fn main() {
    let cmd = Command::parse();
    cmd.run()
}
