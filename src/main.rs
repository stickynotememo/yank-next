// TODO: Clipboard history
// TODO: Prevent deletion of yanked files
// TODO: Directory support
// TODO: Add tests

use clap::error::ErrorKind;
use clap::{Command, CommandFactory, Parser};
use preferences::{AppInfo, Preferences};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path;
use std::path::PathBuf;

#[derive(Parser)]
#[command(bin_name = "yank", author = "stickynotememo", version = "0.1.0", about = "A command line tool to copy and paste files in a desktop-like fashion.", long_about = None)]
struct Args {
    file: Option<String>,

    #[arg(short = 'x', long)]
    cut: bool,

    #[arg(short, long)]
    recursive: bool,

    #[arg(short, long = "paste")]
    paste_file: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Clone, Copy, Debug)]
enum MoveOp {
    Copy,
    Move,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct UserData {
    moveop: MoveOp,
    object_path: PathBuf,
}

const PREFS_KEY: &str = "/etc/yank/clipboardd";
const APP_INFO: AppInfo = AppInfo {
    name: "yank",
    author: "stickynotememo",
};

fn copy(args: &Args, cmd: &mut Command) -> Result<(), clap::error::Error> {
    let Some(file_argument) = &args.file.as_ref() else {
        let err = cmd.error(
            ErrorKind::TooFewValues,
            "no file or folder in clipboard and none specified.",
        );
        return Err(err);
    };
    let copy_file_path = PathBuf::from(file_argument);
    let Ok(file_metadata) = fs::metadata(&copy_file_path) else {
        let err = cmd.error(ErrorKind::Io, "couldn't find file at the path specified.");
        return Err(err);
    };

    let moveop = match args.cut {
        true => MoveOp::Move,
        false => MoveOp::Copy,
    };

    if file_metadata.is_file() {
        // Copy only specifies the file and saves it in storage. The file operation is
        // done by paste()
        let user_data = UserData {
            moveop: moveop,
            object_path: path::absolute(copy_file_path).expect("unexpected error: file seems to have been deleted, even though it was detected to exist. Please file a bug report.")
        };
        let Ok(_) = user_data.save(&APP_INFO, PREFS_KEY) else {
            let err = cmd.error(
                ErrorKind::Io,
                "couldn't access clipboard. Permissions issue?",
            );
            return Err(err);
        };
    } else {
        todo!();
        // Use when directory copy is implemented
        // panic!("yank: Cannot copy \'{}\': Is a directory\nyank: Use -r to copy directories recursively", filename);
    };
    Ok(())
}

fn paste(args: &Args, cmd: &mut Command) -> Result<(), clap::error::Error> {
    // No file specified, yank should paste the file in the clipboard
    // Optionally, the --paste flag can be used to specify where to save the file

    let Ok(user_data) = UserData::load(&APP_INFO, PREFS_KEY) else {
        return Err(cmd.error(
            ErrorKind::Io,
            "no file or folder in clipboard and none specified.",
        ));
    };
    let moveop = user_data.moveop;
    let clipboard = user_data.object_path;

    let paste_file_name: PathBuf = match &args.paste_file {
        // If a paste file has been specified using the flag, it should be used instead
        // of the file used in the clipboard
        // If the clipboard value is being used, the file/directory should be pasted in the
        // current directory while maintaining its filename
        Some(paste_file) => PathBuf::from(paste_file),
        None => PathBuf::from(
            path::absolute(clipboard.file_name().expect(
                "unexpected error: couldn't find clipboard path. Please file a bug report.",
            ))
            .unwrap(),
        ), // TODO: Do something about unwrap.
    };

    let Ok(paste_file_path) = path::absolute(paste_file_name) else {
        return Err(cmd.error(ErrorKind::InvalidValue, "couldn't parse file path"));
    };

    let Ok(_) = fs::metadata(&clipboard) else {
        let err = cmd.error(ErrorKind::Io, "couldn't find file at the path specified.");
        return Err(err);
    };

    match moveop {
        MoveOp::Move => match fs::rename(clipboard, paste_file_path) {
            Ok(_) => Ok(()),
            Err(_) => Err(cmd.error(ErrorKind::Io, "an error occurred while moving files.")),
        },
        MoveOp::Copy => match fs::copy(clipboard, paste_file_path) {
            Ok(_) => Ok(()),
            Err(_) => Err(cmd.error(ErrorKind::Io, "an error occurred while copying files.")),
        },
    }
}

fn main() {
    let args = Args::parse();
    let mut cmd = Args::command();
    let result = match &args.file {
        Some(_) => copy(&args, &mut cmd),
        None => paste(&args, &mut cmd),
    };

    match result {
        Ok(_) => {}
        Err(e) => {
            e.exit();
        }
    };
}
