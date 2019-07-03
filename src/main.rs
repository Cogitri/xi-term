#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![cfg_attr(feature = "clippy", deny(clippy))]

#[macro_use]
extern crate clap;

extern crate failure;

#[macro_use]
extern crate log;
extern crate log4rs;

extern crate futures;
extern crate termion;
extern crate tokio;
extern crate xdg;
extern crate xrl;
extern crate indexmap;

mod core;
mod widgets;

use failure::{Error, ResultExt};
use futures::{Future, Stream};
use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};
use xrl::spawn;

use core::{Tui, TuiServiceBuilder, Command};
use std::io::{stdin, Read, Write};

fn configure_logs(logfile: &str) {
    let tui = FileAppender::builder().build(logfile).unwrap();
    let rpc = FileAppender::builder()
        .build(&format!("{}.rpc", logfile))
        .unwrap();
    let config = Config::builder()
        .appender(Appender::builder().build("tui", Box::new(tui)))
        .appender(Appender::builder().build("rpc", Box::new(rpc)))
        .logger(
            Logger::builder()
                .appender("tui")
                .additive(false)
                .build("xi_tui", LevelFilter::Debug),
        )
        .logger(
            Logger::builder()
                .appender("tui")
                .additive(false)
                .build("xrl", LevelFilter::Info),
        )
        .logger(
            Logger::builder()
                .appender("rpc")
                .additive(false)
                .build("xrl::protocol::codec", LevelFilter::Trace),
        )
        .build(Root::builder().appender("tui").build(LevelFilter::Info))
        .unwrap();
    let _ = log4rs::init_config(config).unwrap();
}

fn main() {
    if let Err(ref e) = run() {
        let stderr = &mut ::std::io::stderr();

        writeln!(stderr, "error: {}", e).unwrap();
        error!("error: {}", e);

        writeln!(stderr, "caused by: {}", e.as_fail()).unwrap();
        error!("error: {}", e);

        writeln!(stderr, "backtrace: {:?}", e.backtrace()).unwrap();
        error!("error: {}", e);

        ::std::process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    let xi = clap_app!(
        xi =>
        (about: "The Xi Editor")
        (@arg core: -c --core +takes_value "Specify binary to use for the backend")
        (@arg logfile: -l --logfile +takes_value "Log file location")
        (@arg file: "File to edit"));

    let matches = xi.get_matches();
    if let Some(logfile) = matches.value_of("logfile") {
        configure_logs(logfile);
    }

    let stdin_string = if matches.value_of("file").is_none() {
        let mut buffer = String::new();
        stdin().read_to_string(&mut buffer)?;
        Some(buffer)
    } else {
        None
    };


    info!("starting xi-core");
    let (tui_builder, core_events_rx) = TuiServiceBuilder::new();
    let (client, core_stderr) = spawn(matches.value_of("core").unwrap_or("xi-core"), tui_builder);

    let error_logging = core_stderr
        .for_each(|msg| {
            error!("core: {}", msg);
            Ok(())
        })
        .map_err(|_| ());
    ::std::thread::spawn(move || {
        tokio::run(error_logging);
    });

    info!("starting logging xi-core errors");

    info!("initializing the TUI");
    let tui = Tui::new(client, core_events_rx);

    let (mut tui, out_rx) = tui.context("Failed to initialize the TUI")?;

    if let Some(user_input) = stdin_string {
        let mut temp_file = tempfile::NamedTempFile::new()?;
        temp_file.write_all(user_input.as_bytes())?;
        tui.handle_cmd(Command::Open(temp_file.path().as_os_str().to_str().map(ToString::to_string)));
    } else {
        tui.handle_cmd(Command::Open(matches.value_of("file").map(ToString::to_string)));
    }
    tui.handle_cmd(Command::SetTheme("base16-eighties.dark".into()));

    info!("spawning the TUI on the event loop");
    tokio::run(tui.map_err(|err| {
        error!("{}", err);
    }));

    if let Ok(out) = out_rx.recv() {
        println!("{}", out);
    }

    Ok(())
}
