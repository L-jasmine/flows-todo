use std::collections::HashMap;

use serde_json::{Result, Value};
use webhook_flows::{
    create_endpoint, request_handler,
    route::{delete, get, options, post, put, route, RouteError, Router},
    send_response,
};

use mysql_async::{
    prelude::*, Conn, Opts, OptsBuilder, Pool, PoolConstraints, PoolOpts, Result, SslOpts,
};

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    create_endpoint().await;
}

#[request_handler]
async fn handler() {
    let mut router = Router::new();
    router
        .insert("/tasks", vec![get(query), post(add_tasks)])
        .unwrap();
    router
        .insert("/tasks/:id", vec![put(update_tasks), delete(delete_tasks)])
        .unwrap();
}

fn get_db_url() -> String {
    if let Ok(url) = std::env::var("DATABASE_URL") {
        let opts = Opts::from_url(&url).expect("DATABASE_URL invalid");
        if opts
            .db_name()
            .expect("a database name is required")
            .is_empty()
        {
            panic!("database name is empty");
        }
        url
    } else {
        "mysql://root:pass@127.0.0.1:3306/mysql".into()
    }
}

async fn get_conn() -> Result<Conn, mysql_async::Error> {
    let opts = Opts::from_url(&*get_url())?;
    let mut builder = OptsBuilder::from_opts(opts);
    builder.ssl_opts(SslOpts::default());
    Conn::new(opts).await
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Task {
    id: u32,
    description: String,
    completed: bool,
}

async fn add_tasks(_headers: Vec<(String, String)>, _qry: HashMap<String, Value>, body: Task) {
    let mut conn = get_conn().await.unwrap();

    match r"insert into tasks (id,description,completed) values (:id,:description,:completed)"
        .with(params! {"id"=>body.id,"description"=>body.description,":completed"=>body.completed})
        .ignore(&mut conn)
        .await
    {
        Ok(_) => send_response(
            200,
            vec![(
                String::from("content-type"),
                String::from("application/json; charset=UTF-8"),
            )],
            serde_json::to_string(&task).unwrap(),
        ),
        Err(e) => send_response(
            200,
            vec![(
                String::from("content-type"),
                String::from("application/json; charset=UTF-8"),
            )],
            serde_json::json!({"err":e.to_string()}).to_string(),
        ),
    }
}

async fn update_tasks(
    _headers: Vec<(String, String)>,
    id: u32,
    _qry: HashMap<String, Value>,
    mut body: Task,
) {
    body.id = id;
    let mut conn = get_conn().await.unwrap();

    match r"update tasks set description= :description,completed=:completed where id = :id"
        .with(params! {"id"=>id,"description"=>body.description,":completed"=>body.completed})
        .ignore(&mut conn)
        .await
    {
        Ok(_) => send_response(
            200,
            vec![(
                String::from("content-type"),
                String::from("application/json; charset=UTF-8"),
            )],
            serde_json::to_string(&task).unwrap(),
        ),
        Err(e) => send_response(
            200,
            vec![(
                String::from("content-type"),
                String::from("application/json; charset=UTF-8"),
            )],
            serde_json::json!({"err":e.to_string()}).to_string(),
        ),
    }
}

async fn delete_tasks(_headers: Vec<(String, String)>, id: u32, _qry: HashMap<String, Value>) {
    let mut conn = get_conn().await.unwrap();

    match r"delete from tasks where id = :id"
        .with(params! {"id"=>id})
        .ignore(&mut conn)
        .await
    {
        Ok(_) => send_response(
            200,
            vec![(
                String::from("content-type"),
                String::from("application/json; charset=UTF-8"),
            )],
            serde_json::to_string(&task).unwrap(),
        ),
        Err(e) => send_response(
            200,
            vec![(
                String::from("content-type"),
                String::from("application/json; charset=UTF-8"),
            )],
            serde_json::json!({"err":e.to_string()}).to_string(),
        ),
    }
}

async fn query(_headers: Vec<(String, String)>, _qry: HashMap<String, Value>, _body: Vec<u8>) {
    let mut conn = get_conn().await.unwrap();

    let tasks = r"select * from tasks"
        .with(())
        .map(&mut conn, |(id, description, completed)| Task {
            id,
            description,
            completed,
        })
        .await
        .unwrap();

    send_response(
        200,
        vec![(
            String::from("content-type"),
            String::from("application/json; charset=UTF-8"),
        )],
        serde_json::to_string(&task).unwrap(),
    )
}
