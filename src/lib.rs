#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate serde_derive;

#[macro_use]
pub mod util;

pub mod errors;
mod schema;

embed_migrations!("migrations");

use crate::{
    schema::{deps, todos},
    util::{blocking, Result},
};
use antidote::Mutex;
use diesel::{
    dsl::{delete, insert_into, update},
    prelude::*,
};
use futures::Future;
use std::{error::Error, sync::Arc};

/// A database connection.
#[derive(Clone)]
pub struct DB {
    db: Arc<Mutex<SqliteConnection>>,
}

impl DB {
    /// Connects to the SQLite database.
    pub fn connect(database_path: &str) -> Result<DB> {
        let db = SqliteConnection::establish(database_path)?;
        embedded_migrations::run_with_output(&db, &mut std::io::stderr())?;
        Ok(DB {
            db: Arc::new(Mutex::new(db)),
        })
    }

    /// Gets the contents of the database.
    pub fn get_all(
        &self,
    ) -> impl Future<Item = GetAll, Error = Box<dyn Error + Send + Sync + 'static>> {
        let db = self.db.clone();
        blocking(move || -> Result<_> {
            let db = db.lock();
            let todos = todos::table.get_results::<Todo>(&*db)?;
            let deps = deps::table
                .select((deps::id_from, deps::id_to))
                .get_results::<Dep>(&*db)?;
            Ok(GetAll { todos, deps })
        })
    }

    /// Creates a new dependency.
    pub fn create_dep(
        &self,
        dep: Dep,
    ) -> impl Future<Item = (), Error = Box<dyn Error + Send + Sync + 'static>> {
        let db = self.db.clone();
        blocking(move || -> Result<_> {
            let db = db.lock();
            insert_into(deps::table)
                .values((deps::id_from.eq(dep.from), deps::id_to.eq(dep.to)))
                .execute(&*db)?;
            Ok(())
        })
    }

    /// Deletes a dependency.
    pub fn delete_dep(
        &self,
        dep: Dep,
    ) -> impl Future<Item = (), Error = Box<dyn Error + Send + Sync + 'static>> {
        let db = self.db.clone();
        blocking(move || -> Result<_> {
            let db = db.lock();
            delete(deps::table)
                .filter(deps::id_from.eq(dep.from))
                .filter(deps::id_to.eq(dep.to))
                .execute(&*db)?;
            Ok(())
        })
    }

    /// Creates a Todo.
    pub fn create_todo(
        &self,
        req: CreateTodo,
    ) -> impl Future<Item = (), Error = Box<dyn Error + Send + Sync + 'static>> {
        let db = self.db.clone();
        blocking(move || -> Result<_> {
            let db = db.lock();
            insert_into(todos::table)
                .values(todos::name.eq(req.name))
                .execute(&*db)?;
            Ok(())
        })
    }

    /// Modifies a Todo.
    pub fn modify_todo(
        &self,
        todo: Todo,
    ) -> impl Future<Item = (), Error = Box<dyn Error + Send + Sync + 'static>> {
        let db = self.db.clone();
        blocking(move || -> Result<_> {
            let db = db.lock();
            update(todos::table)
                .filter(todos::id.eq(todo.id))
                .set((todos::name.eq(todo.name), todos::done.eq(todo.done)))
                .execute(&*db)?;
            Ok(())
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTodo {
    name: String,
}

#[derive(Debug, Serialize)]
pub struct GetAll {
    todos: Vec<Todo>,
    deps: Vec<Dep>,
}

#[derive(Debug, Deserialize, Queryable, Serialize)]
pub struct Dep {
    from: i32,
    to: i32,
}

#[derive(Debug, Deserialize, Queryable, Serialize)]
pub struct Todo {
    id: i32,
    name: String,
    done: bool,
}
