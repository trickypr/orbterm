#[macro_use]
extern crate serde_derive;
#[cfg(feature = "env_logger")]
extern crate env_logger;
extern crate failure;
extern crate orbclient;
extern crate orbfont;
extern crate toml;
extern crate xdg;

#[cfg(not(target_os = "redox"))]
extern crate libc;

#[cfg(target_os = "redox")]
extern crate redox_termios;

#[cfg(target_os = "redox")]
extern crate syscall;

use std::io::Write;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::{cmp, env, io};

use before_exec::before_exec;
use config::Config;
use console::Console;
use getpty::getpty;
use handle::handle;
use slave_stdio::slave_stdio;

mod before_exec;
mod block_handler;
mod config;
mod console;
mod getpty;
mod handle;
mod slave_stdio;

pub const BLOCK_WIDTH: u32 = 8;
pub const BLOCK_HEIGHT: u32 = BLOCK_WIDTH * 2;

const DEFAULT_INITIAL_WIDTH: u32 = 80;
const DEFAULT_INITIAL_HEIGHT: u32 = 24;

fn main() {
    #[cfg(feature = "env_logger")]
    env_logger::init();

    let config = match Config::load() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("orbterm: failed to open config: {}", err);
            return;
        }
    };

    let mut args = env::args().skip(1);

    let user_specified_shell = args.next();
    let system_shell = env::var("SHELL").unwrap_or("/bin/sh".to_string());

    let shell = user_specified_shell.unwrap_or(system_shell);

    let (_display_width, display_height) =
        orbclient::get_display_size().expect("terminal: failed to get display size");

    let columns = config.columns.unwrap_or(DEFAULT_INITIAL_WIDTH);
    let rows = config.rows.unwrap_or(DEFAULT_INITIAL_HEIGHT);

    let (master_fd, tty_path) = getpty(columns, rows);
    let (slave_stdin, slave_stdout, slave_stderr) =
        slave_stdio(&tty_path).expect("terminal: failed to get slave stdio");

    let mut command = Command::new(&shell);
    for arg in args {
        command.arg(arg);
    }

    command
        // Not setting COLUMNS and LINES fixes many applications that use it
        // to quickly get the current terminal size instead of TIOCSWINSZ
        .env("COLUMNS", "")
        .env("LINES", "")
        // It is useful to know if we are running inside of orbterm, some times
        .env("ORBTERM_VERSION", env!("CARGO_PKG_VERSION"))
        // We emulate xterm-256color
        .env("TERM", "xterm-256color")
        .env("TTY", tty_path);

    unsafe {
        command
            .stdin(Stdio::from_raw_fd(slave_stdin.as_raw_fd()))
            .stdout(Stdio::from_raw_fd(slave_stdout.as_raw_fd()))
            .stderr(Stdio::from_raw_fd(slave_stderr.as_raw_fd()))
            .pre_exec(|| before_exec());
    }

    match command.spawn() {
        Ok(mut process) => {
            drop(slave_stderr);
            drop(slave_stdout);
            drop(slave_stdin);

            let scale = config
                .get_initial_scale(display_height)
                .expect("Failed to retrieve the default scale");
            let (block_width, block_height) = (
                (BLOCK_WIDTH as f32 * scale) as u32,
                (BLOCK_HEIGHT as f32 * scale) as u32,
            );

            let mut console = Console::new(
                &config,
                columns * block_width as u32,
                rows * block_height as u32,
                block_width as usize,
                block_height as usize,
            );

            handle(&mut console, master_fd, &mut process);
        }
        Err(err) => {
            let term_stderr = io::stderr();
            let mut term_stderr = term_stderr.lock();
            let _ = writeln!(
                term_stderr,
                "terminal: failed to execute '{}': {:?}",
                shell, err
            );
        }
    }
}
