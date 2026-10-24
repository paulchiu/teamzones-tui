use assert_cmd::Command;

#[test]
fn version_flag_prints_the_binary_name_and_package_version() {
    let expected = format!("teamzones {}\n", env!("CARGO_PKG_VERSION"));
    Command::cargo_bin("teamzones")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(expected);
}
