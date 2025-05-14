use std::io::Write;
use std::process::{Command, Stdio};

/// Spawns a fzf process to allow the user to select an item from a list. Then returns the selected item.
pub fn fuzzy_search(items: Vec<String>) -> std::io::Result<String> {
    let mut fzf = Command::new("fzf")
        .arg("--style=full")
        .arg("--border-label=MDN Docs")
        .arg("--input-label=Search")
        .arg("--color=bg+:black,fg+:white,hl+:yellow,preview-bg:235,preview-fg:white")
        .arg("--preview=barbara preview {}") // TODO: Went built this should call the PATH cmd
        .arg("--preview-window=up:70%:wrap")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    // Push items to fzf's stdin
    {
        let fzf_stdin = fzf.stdin.as_mut().expect("Failed to open fzf stdin");
        for item in &items {
            writeln!(fzf_stdin, "{}", item)?;
        }
    }

    let output = fzf.wait_with_output()?;
    let selection = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if selection.is_empty() {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No selection was made",
        ))
    } else {
        Ok(selection)
    }
}
