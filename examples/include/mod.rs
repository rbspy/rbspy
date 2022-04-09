pub(crate) fn path_to_ruby_binary() -> String {
    let which = get_which();

    let output = std::process::Command::new(which)
        .arg("ruby")
        .output()
        .expect("failed to execute process");

    let ruby_binary_path = String::from_utf8(output.stdout).unwrap();

    let ruby_binary_path_str = ruby_binary_path
        .lines()
        .next()
        .expect("failed to execute ruby process");

    ruby_binary_path_str.to_string()
}

#[cfg(target_os = "windows")]
fn get_which<'a>() -> &'a str {
    "C:\\Windows\\System32\\WHERE.exe"
}

#[cfg(not(target_os = "windows"))]
fn get_which<'a>() -> &'a str {
    "/usr/bin/which"
}
