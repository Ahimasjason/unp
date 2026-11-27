use std::{io, process::Command};




pub(crate) fn run_command(command : &str, args: &[&str]) -> io::Result<()>   {
    let output = Command::new(command)
    .args(args)
    .output()?;

    if !output.status.success(){
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(format!(
            "{command} {args:?} failed ({}): {}",
            output.status,
            stderr.trim_end()
        )));
    }
    Ok(())
}
