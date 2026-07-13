//! End-to-end check of the compiled binary: arg parsing through match
//! resolution to the summary line.

use std::process::Command;

fn overthrow() -> Command {
    Command::new(env!("CARGO_BIN_EXE_overthrow"))
}

#[test]
fn match_subcommand_runs_to_completion() {
    let output = overthrow()
        .args(["match", "--games", "2"])
        .output()
        .expect("failed to spawn binary");
    assert!(output.status.success(), "exit: {:?}", output.status);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("2 games"), "unexpected output: {stdout}");
}

#[test]
fn six_bots_run_a_six_player_match() {
    let output = overthrow()
        .args([
            "match",
            "--games",
            "2",
            "--radius",
            "6",
            "--bots",
            "greedy,greedy,greedy,random,random,random",
        ])
        .output()
        .expect("failed to spawn binary");
    assert!(output.status.success(), "exit: {:?}", output.status);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("6 players"), "unexpected output: {stdout}");
    // One standings entry per seat, P0 through P5.
    assert!(stdout.contains("[P5]"), "unexpected output: {stdout}");
}

#[test]
fn too_many_bots_are_rejected() {
    let output = overthrow()
        .args([
            "match",
            "--bots",
            "greedy,greedy,greedy,greedy,greedy,greedy,greedy",
        ])
        .output()
        .expect("failed to spawn binary");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn unknown_arguments_are_rejected() {
    let output = overthrow()
        .args(["match", "--nonsense"])
        .output()
        .expect("failed to spawn binary");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn missing_subcommand_prints_usage() {
    let output = overthrow().output().expect("failed to spawn binary");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("usage:"), "unexpected stderr: {stderr}");
}
