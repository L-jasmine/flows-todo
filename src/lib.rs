use std::collections::HashMap;

use serde_json::Value;
use webhook_flows::{
    create_endpoint, request_handler,
    route::{delete, get, post, put, route, RouteError, Router},
    send_response,
};

use mysql_async::{prelude::*, Conn, Opts, OptsBuilder, Result, SslOpts};

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    create_endpoint().await;
}

#[request_handler]
async fn handler() {
    flowsnet_platform_sdk::logger::init();
    let mut router = Router::new();
    router
        .insert("/tasks", vec![get(query), post(add_tasks)])
        .unwrap();
    router
        .insert("/tasks/:id", vec![put(update_tasks), delete(delete_tasks)])
        .unwrap();

    if let Err(e) = route(router).await {
        match e {
            RouteError::NotFound => {
                send_response(
                    404,
                    vec![],
                    serde_json::to_vec(&serde_json::json!({"err":"No route matched"})).unwrap(),
                );
            }
            RouteError::MethodNotAllowed => {
                send_response(
                    405,
                    vec![],
                    serde_json::to_vec(&serde_json::json!({"err":"Method not allowed"})).unwrap(),
                );
            }
        }
    }
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

async fn get_conn() -> Result<Conn> {
    let db_url = get_db_url();
    log::debug!("connect db {db_url}");
    let opts = Opts::from_url(&db_url)?;
    let builder = OptsBuilder::from_opts(opts);
    let builder = builder.ssl_opts(SslOpts::default());
    Conn::new(builder).await
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Task {
    #[serde(default)]
    id: u32,
    description: String,
    completed: bool,
}

async fn add_tasks(_headers: Vec<(String, String)>, _qry: HashMap<String, Value>, body: Vec<u8>) {
    let mut conn = get_conn().await.unwrap();

    let task: Task = serde_json::from_slice(&body).unwrap();

    match r"insert into tasks (description,completed) values (:description,:completed)"
        .with(params! {"description"=>&task.description,"completed"=>&task.completed})
        .ignore(&mut conn)
        .await
    {
        Ok(_) => send_response(
            200,
            vec![(
                String::from("content-type"),
                String::from("application/json; charset=UTF-8"),
            )],
            serde_json::to_vec(&task).unwrap(),
        ),
        Err(e) => send_response(
            200,
            vec![(
                String::from("content-type"),
                String::from("application/json; charset=UTF-8"),
            )],
            serde_json::json!({"err":e.to_string()}).to_string().into(),
        ),
    }
}

async fn update_tasks(_headers: Vec<(String, String)>, qry: HashMap<String, Value>, body: Vec<u8>) {
    let id = qry.get("id").unwrap().as_u64().unwrap() as u32;
    let mut task: Task = serde_json::from_slice(&body).unwrap();

    task.id = id;
    let mut conn = get_conn().await.unwrap();

    match r"update tasks set description= :description,completed=:completed where id = :id"
        .with(params! {"id"=>id,"description"=>&task.description,"completed"=>&task.completed})
        .ignore(&mut conn)
        .await
    {
        Ok(_) => send_response(
            200,
            vec![(
                String::from("content-type"),
                String::from("application/json; charset=UTF-8"),
            )],
            serde_json::to_vec(&task).unwrap(),
        ),
        Err(e) => send_response(
            200,
            vec![(
                String::from("content-type"),
                String::from("application/json; charset=UTF-8"),
            )],
            serde_json::json!({"err":e.to_string()}).to_string().into(),
        ),
    }
}

async fn delete_tasks(
    _headers: Vec<(String, String)>,
    qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    let id = qry.get("id").unwrap().as_u64().unwrap() as u32;

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
            serde_json::json!({"id":id}).to_string().into(),
        ),
        Err(e) => send_response(
            200,
            vec![(
                String::from("content-type"),
                String::from("application/json; charset=UTF-8"),
            )],
            serde_json::json!({"err":e.to_string()}).to_string().into(),
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
        serde_json::to_vec(&tasks).unwrap(),
    )
}
