#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate serde_derive;

#[macro_use]
mod util;

mod errors;
mod options;
mod schema;

embed_migrations!("migrations");

use crate::{
    errors::BadAuth,
    options::Options,
    schema::{deps, todos},
    util::{blocking, log_err, Result},
};
use antidote::Mutex;
use diesel::{
    dsl::{delete, insert_into, update},
    prelude::*,
};
use futures::Future;
use log::warn;
use packer::Packer;
use std::{path::Path, process::exit, sync::Arc};
use structopt::StructOpt;
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
    let db = SqliteConnection::establish(&options.database_path)?;
    embedded_migrations::run_with_output(&db, &mut std::io::stderr())?;

    let authorization = Box::leak(options.authorization().into_boxed_str());
    let routes = warp::header::exact("authorization", authorization)
        .or_else(|_| Err(warp::reject::custom(BadAuth)))
        .and(routes(Arc::new(Mutex::new(db))))
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

fn routes(
    db: Arc<Mutex<SqliteConnection>>,
) -> impl Clone + Filter<Extract = (impl warp::Reply,), Error = Rejection> {
    #[derive(Debug, Deserialize)]
    struct CreateTodo {
        name: String,
    }

    #[derive(Debug, Serialize)]
    struct GetAll {
        todos: Vec<Todo>,
        deps: Vec<Dep>,
    }

    #[derive(Debug, Deserialize, Queryable, Serialize)]
    struct Dep {
        from: i32,
        to: i32,
    }

    #[derive(Debug, Deserialize, Queryable, Serialize)]
    struct Todo {
        id: i32,
        name: String,
        done: bool,
    }

    let get_all_db = db.clone();
    let get_all = path!("api" / "all")
        .and(warp::path::end())
        .and(warp::get2())
        .and_then(move || {
            let db = get_all_db.clone();
            blocking(move || -> Result<_> {
                let db = db.lock();
                let todos = todos::table.get_results::<Todo>(&*db)?;
                let deps = deps::table
                    .select((deps::id_from, deps::id_to))
                    .get_results::<Dep>(&*db)?;
                Ok(GetAll { todos, deps })
            })
            .map_err(warp::reject::custom)
        })
        .map(|r| warp::reply::json(&r));

    let create_dep_db = db.clone();
    let create_dep = path!("api" / "dep")
        .and(warp::path::end())
        .and(warp::post2())
        .and(warp::body::json())
        .and_then(move |dep: Dep| {
            let db = create_dep_db.clone();
            blocking(move || -> Result<_> {
                let db = db.lock();
                insert_into(deps::table)
                    .values((deps::id_from.eq(dep.from), deps::id_to.eq(dep.to)))
                    .execute(&*db)?;
                Ok(())
            })
            .map_err(warp::reject::custom)
        })
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
        .and_then(move |dep: Dep| {
            let db = delete_dep_db.clone();
            blocking(move || -> Result<_> {
                let db = db.lock();
                delete(deps::table)
                    .filter(deps::id_from.eq(dep.from))
                    .filter(deps::id_to.eq(dep.to))
                    .execute(&*db)?;
                Ok(())
            })
            .map_err(warp::reject::custom)
        })
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
            let db = create_todo_db.clone();
            blocking(move || -> Result<_> {
                let db = db.lock();
                insert_into(todos::table)
                    .values(todos::name.eq(req.name))
                    .execute(&*db)?;
                Ok(())
            })
            .map_err(warp::reject::custom)
        })
        .and_then(|()| {
            Response::builder()
                .status(204)
                .body("")
                .map_err(warp::reject::custom)
        });

    let modify_todo_db = db.clone();
    let modify_todo = path!("api" / "new-todo")
        .and(warp::path::end())
        .and(warp::post2())
        .and(warp::body::json())
        .and_then(move |todo: Todo| {
            let db = modify_todo_db.clone();
            blocking(move || -> Result<_> {
                let db = db.lock();
                update(todos::table)
                    .filter(todos::id.eq(todo.id))
                    .set((todos::name.eq(todo.name), todos::done.eq(todo.done)))
                    .execute(&*db)?;
                Ok(())
            })
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
