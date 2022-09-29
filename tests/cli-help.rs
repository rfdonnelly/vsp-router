use snapbox::cmd::{cargo_bin, Command};

#[test]
fn long() {
    Command::new(cargo_bin!("vsp-router"))
        .arg("--help")
        .assert()
        .success()
        .stderr_eq("")
        .stdout_matches_path("tests/snapshots/cli-help-long.txt");
}
