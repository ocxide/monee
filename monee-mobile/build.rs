use std::process::Command;

fn main() {
    Command::new("bunx")
        .args(["tailwindcss", "-i", "styles.css", "-o", "output.css"])
        .output()
        .unwrap();
}
