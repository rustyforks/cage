//! Our main CLI tool.

#[macro_use]
extern crate conductor;
extern crate docopt;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate rustc_serialize;

use docopt::Docopt;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

use conductor::command_runner::OsCommandRunner;
use conductor::cmd::*;
use conductor::Error;

/// Our version number, set by Cargo at compile time.
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

/// Our help string.
const USAGE: &'static str = "
conductor: Manage large, multi-pod docker-compose apps

Usage:
  conductor [options]
  conductor [options] pull
  conductor [options] up
  conductor [options] stop
  conductor [options] repo list
  conductor [options] repo clone <repo>
  conductor (--help | --version)

Commands:
  pull              Pull Docker images used by project
  up                Run project
  stop              Stop all containers associated with project
  repo list         List all git repository aliases and URLs
  repo clone        Clone a git repository using its short alias and mount it
                    into the containers that use it

Arguments:
  <repo>            Short alias for a repo (see `repo list`)

Options:
  -h, --help        Show this message
  --version         Show the version of conductor
  --override=<override>
                    Use overrides from the specified subdirectory of
                    `pods/overrides` [default: development]
  --default-tags=<tag_file>
                    A list of tagged image names, one per line.  Tags
                    will be used as defaults for those images.

Run conductor in a directory containing a `pods` subdirectory.  For more
information, see https://github.com/faradayio/conductor.
";

/// Our parsed command-line arguments.  See [docopt.rs][] for an
/// explanation of how this works.
///
/// [docopt.rs]: https://github.com/docopt/docopt.rs
#[derive(Debug, RustcDecodable)]
struct Args {
    cmd_pull: bool,
    cmd_up: bool,
    cmd_stop: bool,
    cmd_repo: bool,
    cmd_list: bool,
    cmd_clone: bool,

    arg_repo: Option<String>,

    flag_version: bool,
    flag_override: String,
    flag_default_tags: Option<String>
}

/// The function which does the real work.  Unlike `main`, we have a return
/// type of `Result` and may therefore use `try!` to handle errors.
fn run(args: &Args) -> Result<(), Error> {
    let mut proj = try!(conductor::Project::from_current_dir());
    if let Some(ref default_tags_path) = args.flag_default_tags {
        let file = try!(fs::File::open(default_tags_path));
        proj.set_default_tags(try!(conductor::DefaultTags::read(file)));
    }
    let ovr = try!(proj.ovr(&args.flag_override).ok_or_else(|| {
        err!("override {} is not defined", &args.flag_override)
    }));
    try!(proj.output());
    let runner = OsCommandRunner;

    if args.cmd_pull {
        try!(proj.pull(&runner, &ovr));
    } else if args.cmd_up {
        try!(proj.up(&runner, &ovr));
    } else if args.cmd_stop {
        try!(proj.stop(&runner, &ovr));
    } else if args.cmd_repo && args.cmd_list {
        try!(proj.repo_list(&runner));
    } else if args.cmd_repo && args.cmd_clone {
        try!(proj.repo_clone(&runner, args.arg_repo.as_ref().unwrap()));
        // Regenerate our output now that we've cloned.
        try!(proj.output());
    }

    Ok(())
}

/// Our main entry point.
fn main() {
    // Initialize logging with some custom options, mostly so we can see
    // our own warnings.
    let mut builder = env_logger::LogBuilder::new();
    builder.filter(Some("docker_compose"), log::LogLevelFilter::Warn);
    builder.filter(Some("conductor"), log::LogLevelFilter::Warn);
    if let Ok(config) = env::var("RUST_LOG") {
        builder.parse(&config);
    }
    builder.init().unwrap();

    // Parse our args using docopt.rs.
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    debug!("Arguments: {:?}", &args);

    // Display our version if we were asked to do so.
    if args.flag_version {
        println!("conductor {}", VERSION);
        process::exit(0);
    }

    // Defer all our real work to `run`, and handle any errors.  This is a
    // standard Rust pattern to make error-handling in `main` nicer.
    if let Err(ref err) = run(&args) {
        // We use `unwrap` here to turn I/O errors into application panics.
        // If we can't print a message to stderr without an I/O error,
        // the situation is hopeless.
        write!(io::stderr(), "Error: {}\n", err).unwrap();
        process::exit(1);
    }
}
