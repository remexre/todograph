use crate::{errors::ErrorString, util::Result};
use std::net::{SocketAddr, ToSocketAddrs};

#[derive(Debug, structopt::StructOpt)]
#[structopt(raw(setting = "::structopt::clap::AppSettings::ColoredHelp"))]
pub struct Options {
    /// Turns off message output.
    #[structopt(short = "q", long = "quiet")]
    quiet: bool,

    /// Increases the verbosity.
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,

    /// The name of the SQLite database file.
    #[structopt(long = "db", env = "DATABASE_PATH", default_value = "todograph.db")]
    pub database_path: String,

    /// The host to serve on.
    #[structopt(short = "H", long = "host", env = "HOST", default_value = "::")]
    host: String,

    /// The port to serve on.
    #[structopt(short = "P", long = "port", env = "PORT", default_value = "8080")]
    port: u16,

    /// The password to require for auth.
    #[structopt(long = "password", env = "PASSWORD")]
    pub password: String,

    /// The username to require for auth.
    #[structopt(long = "username", env = "USERNAME")]
    pub username: String,
}

impl Options {
    /// Gets the `Authorization` header to accept.
    pub fn authorization(&self) -> String {
        let mut s = "Basic ".to_string();
        let creds = format!("{}:{}", self.username, self.password);
        base64::encode_config_buf(&creds, base64::STANDARD, &mut s);
        s
    }

    /// Get the address to serve on.
    pub fn serve_addr(&self) -> Result<SocketAddr> {
        let addrs = (&self.host as &str, self.port)
            .to_socket_addrs()?
            .collect::<Vec<_>>();
        if addrs.is_empty() {
            return Err(Box::new(ErrorString(
                "No matching address exists".to_string(),
            )));
        } else {
            Ok(addrs[0])
        }
    }

    /// Sets up logging as specified by the `-q` and `-v` flags.
    pub fn start_logger(&self) {
        stderrlog::new()
            .quiet(self.quiet)
            .verbosity(self.verbose + 2)
            .init()
            .unwrap()
    }
}
