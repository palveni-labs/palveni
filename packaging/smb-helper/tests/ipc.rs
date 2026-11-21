use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

fn run(frame: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_palveni-smb-helper"))
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(frame).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn rejects_an_unbounded_command_without_echoing_input() {
    let out = run(&u32::MAX.to_be_bytes());
    assert!(!out.status.success());
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
}

#[test]
fn refuses_guest_before_connecting_and_keeps_credentials_off_both_output_pipes() {
    let body = br#"{"host":"localhost","port":1,"path":"/media","username":"Guest","secret":"example-only","operation":{"op":"list","parent":"","offset":0,"limit":1}}"#;
    let mut frame = (body.len() as u32).to_be_bytes().to_vec();
    frame.extend_from_slice(body);
    let out = run(&frame);
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    let expected = br#"{"status":"invalid"}"#;
    assert_eq!(out.stdout[..4], (expected.len() as u32).to_be_bytes());
    assert_eq!(&out.stdout[4..], expected);
}
