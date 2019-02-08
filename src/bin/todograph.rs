#[macro_use]
extern crate todograph;

use futures::Future;
use log::warn;
use packer::Packer;
use std::{
    net::{SocketAddr, ToSocketAddrs},
    path::Path,
    process::exit,
};
use structopt::StructOpt;
use todograph::{
    errors::{BadAuth, ErrorString},
    util::{log_err, Result},
    CreateTodo, Dep, Todo, DB,
};
use warp::{http::Response, path, Filter, Rejection};

fn main() {
    let options = Options::from_args();
    options.start_logger();

    if let Err(err) = run(options) {
        log_err(err.as_ref());
        exit(1);
    }
}

fn run(options: Options) -> Result<()> {
    let serve_addr = options.serve_addr()?;
    let db = DB::connect(&options.database_path)?;

    let authorization = Box::leak(options.authorization().into_boxed_str());
    let routes = warp::header::exact("authorization", authorization)
        .or_else(|_| Err(warp::reject::custom(BadAuth)))
        .and(routes(db))
        .recover(|rej: Rejection| {
            if rej.find_cause::<BadAuth>().is_some() {
                Response::builder()
                    .status(401)
                    .header("www-authenticate", "Basic realm=\"todograph\"")
                    .body("")
                    .map_err(warp::reject::custom)
            } else {
                Err(rej)
            }
        })
        .with(warp::log("todograph"));

    tokio::run(warp::serve(routes).bind(serve_addr));

    Ok(())
}

fn routes(db: DB) -> impl Clone + Filter<Extract = (impl warp::Reply,), Error = Rejection> {
    let get_all_db = db.clone();
    let get_all = path!("api" / "all")
        .and(warp::path::end())
        .and(warp::get2())
        .and_then(move || get_all_db.get_all().map_err(warp::reject::custom))
        .map(|r| warp::reply::json(&r));

    let create_dep_db = db.clone();
    let create_dep = path!("api" / "dep")
        .and(warp::path::end())
        .and(warp::post2())
        .and(warp::body::json())
        .and_then(move |dep: Dep| create_dep_db.create_dep(dep).map_err(warp::reject::custom))
        .and_then(|()| {
            Response::builder()
                .status(204)
                .body("")
                .map_err(warp::reject::custom)
        });

    let delete_dep_db = db.clone();
    let delete_dep = path!("api" / "dep")
        .and(warp::path::end())
        .and(warp::post2())
        .and(warp::body::json())
        .and_then(move |dep: Dep| delete_dep_db.delete_dep(dep).map_err(warp::reject::custom))
        .and_then(|()| {
            Response::builder()
                .status(204)
                .body("")
                .map_err(warp::reject::custom)
        });

    let create_todo_db = db.clone();
    let create_todo = path!("api" / "new-todo")
        .and(warp::path::end())
        .and(warp::post2())
        .and(warp::body::json())
        .and_then(move |req: CreateTodo| {
            create_todo_db
                .create_todo(req)
                .map_err(warp::reject::custom)
        })
        .and_then(|()| {
            Response::builder()
                .status(204)
                .body("")
                .map_err(warp::reject::custom)
        });

    let modify_todo_db = db.clone();
    let modify_todo = path!("api" / "modify-todo")
        .and(warp::path::end())
        .and(warp::post2())
        .and(warp::body::json())
        .and_then(move |todo: Todo| {
            modify_todo_db
                .modify_todo(todo)
                .map_err(warp::reject::custom)
        })
        .and_then(|()| {
            Response::builder()
                .status(204)
                .body("")
                .map_err(warp::reject::custom)
        });

    statics()
        .or(get_all)
        .or(create_dep)
        .or(delete_dep)
        .or(create_todo)
        .or(modify_todo)
}

fn statics() -> impl Clone + Filter<Extract = (Response<Vec<u8>>,), Error = Rejection> {
    use warp::path::Tail;

    #[derive(Packer)]
    #[folder = "src/static"]
    struct Assets;

    let index = warp::path::end().and_then(|| {
        Assets::get("index.html")
            .map(|body| ("index.html".to_string(), body))
            .ok_or_else(warp::reject::not_found)
    });
    let with_path = warp::path::tail().and_then(|path: Tail| {
        let path = path.as_str();
        Assets::get(path)
            .map(|body| (path.to_string(), body))
            .ok_or_else(warp::reject::not_found)
    });
    index
        .or(with_path)
        .unify()
        .and(warp::get2())
        .untuple_one()
        .and_then(|path: String, body: &[u8]| {
            let ext = coerce!(path.as_str().as_ref() => &Path)
                .extension()
                .and_then(|s| s.to_str());
            let ct = match ext {
                Some("css") => "text/css; charset=utf-8",
                Some("html") => "text/html; charset=utf-8",
                Some("js") => "application/javascript",
                _ => {
                    warn!("Unknown extension for static file: {:?}", ext);
                    "application/octet-stream"
                }
            };
            Response::builder()
                .header("content-type", ct)
                .body(body.to_owned())
                .map_err(warp::reject::custom)
        })
}

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
