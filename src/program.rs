//! Starting a configured tool ([`Program`], [`ToolCommand`]): the program itself, or, where `[tools] wine` names a
//! launcher (Wine on Linux), a Windows program through that launcher, with its path arguments written as the
//! launcher's programs see them.
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output};

/// A configured tool's program and how this machine starts it: [`Program::command`] gives a fresh [`ToolCommand`] for
/// each run.
#[derive(Clone, Debug)]
pub struct Program {
    key: String,
    path: PathBuf,
    launcher: Option<PathBuf>,
}

impl Program {
    /// The program `path` (configured as `key`, which a failure names), started through `launcher` when one is given
    /// and the program is a Windows one. A Windows program on a machine that runs none itself and has no launcher
    /// fails, naming the setting.
    #[track_caller]
    pub(crate) fn new(key: &str, path: PathBuf, launcher: Option<PathBuf>) -> Program {
        let launcher = launcher.filter(|_| is_windows_program(&path));
        assert!(
            cfg!(windows) || launcher.is_some() || !is_windows_program(&path),
            "{key} = {} is a Windows program, and this machine runs none itself: set [tools] wine to a launcher \
             that does (Wine's)",
            path.display()
        );
        Program {
            key: key.to_owned(),
            path,
            launcher,
        }
    }

    /// The program's file, as configured.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The configuration key it was found under (`tools.texconv`).
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Whether it runs through the `[tools] wine` launcher.
    pub fn launched(&self) -> bool {
        self.launcher.is_some()
    }

    /// A command that runs it, with no arguments yet.
    pub fn command(&self) -> ToolCommand {
        match &self.launcher {
            Some(launcher) => {
                let mut command = Command::new(launcher);
                command.arg(wine_path(&self.path));
                ToolCommand {
                    command,
                    launched: true,
                }
            }
            None => ToolCommand {
                command: Command::new(&self.path),
                launched: false,
            },
        }
    }
}

/// A command that runs a configured tool ([`Program::command`]): [`std::process::Command`] for a program this machine
/// runs itself, or for its launcher, `<launcher> <program> args...`, when the program is a Windows one (`.exe`) and
/// the configuration's `[tools] wine` names a launcher.
///
/// Arguments that are paths go through [`ToolCommand::path_arg`]: under a launcher an absolute path is written as a
/// Wine path (`/tmp/a.dds` as `Z:\tmp\a.dds`), since a Windows program reads a leading `/` as an option and knows no
/// Unix paths; without one it is passed as it is. Everything else goes through [`ToolCommand::arg`] unchanged. A path
/// inside a file the program reads may stay a Unix one: Wine opens `/tmp/a.dds` on the current drive, which for a
/// program started from a Unix folder is `Z:`, the Unix root.
#[derive(Debug)]
pub struct ToolCommand {
    command: Command,
    launched: bool,
}

impl ToolCommand {
    /// Whether the program runs through the `[tools] wine` launcher.
    pub fn launched(&self) -> bool {
        self.launched
    }

    /// An argument, passed as it is.
    pub fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut ToolCommand {
        self.command.arg(arg);
        self
    }

    /// Arguments, passed as they are.
    pub fn args<I, S>(&mut self, args: I) -> &mut ToolCommand
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    /// A path argument: as a Wine path under a launcher (an absolute path; a relative one is passed as it is), else
    /// as it is.
    pub fn path_arg(&mut self, path: impl AsRef<Path>) -> &mut ToolCommand {
        if self.launched {
            self.command.arg(wine_path(path.as_ref()));
        } else {
            self.command.arg(path.as_ref());
        }
        self
    }

    /// The underlying command, for its environment, working folder and standard streams.
    pub fn command(&mut self) -> &mut Command {
        &mut self.command
    }

    /// Run it and collect its output ([`Command::output`]).
    pub fn output(&mut self) -> std::io::Result<Output> {
        self.command.output()
    }

    /// Run it and wait for it ([`Command::status`]).
    pub fn status(&mut self) -> std::io::Result<ExitStatus> {
        self.command.status()
    }
}

/// Whether `program` is a Windows program: its extension is `exe`, in any case.
fn is_windows_program(program: &Path) -> bool {
    program.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"))
}

/// `path` as Wine names it: an absolute Unix path on drive `Z:` (the Unix root) with `\` separators; any other path as it
/// is.
pub fn wine_path(path: &Path) -> OsString {
    match path.to_str() {
        Some(text) if text.starts_with('/') => OsString::from(format!("Z:{}", text.replace('/', "\\"))),
        _ => path.as_os_str().to_owned(),
    }
}
