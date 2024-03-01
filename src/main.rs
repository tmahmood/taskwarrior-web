use std::collections::HashMap;
use anyhow::Error;
use axum::{Form, Router, routing::get};
use axum::extract::{Multipart, Query};
use axum::response::Html;
use axum::routing::post;
use serde::Deserialize;
use tera::{Context, Tera};
use tracing::{debug, error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use org_me::endpoints::tasks::{list_tasks, Task, update_tasks};
use org_me::{Params, TEMPLATES};


#[tokio::main]
async fn main() {
    // initialize tracing

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "org_me=debug,tower_http=debug".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_line_number(true)
        )
        .init();

    // build our application with a route
    let app = Router::new()
        .route("/", get(front_page))
        .nest_service(
            "/dist",
            tower_http::services::ServeDir::new("./dist"),
        )
        .route("/tasks", get(tasks_display))
        .route("/tasks", post(change_task_status))
        ;

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}


async fn front_page() -> Html<String> {
    let filter = vec!["status:pending"];
    let tasks = list_tasks(filter.clone()).unwrap();
    let mut ctx = Context::new();
    ctx.insert("tasks", &tasks);
    ctx.insert("current_filter", &filter);
    ctx.insert("filter_value", &filter.join(" "));
    Html(TEMPLATES.render("base.html", &ctx).unwrap())
}

async fn tasks_display(Query(params): Query<Params>) -> Html<String> {
    get_tasks_view(params)
}

fn get_tasks_view(params: Params) -> Html<String> {
    let query = params.query();
    let tasks = match list_tasks(query.clone()) {
        Ok(t) => { t }
        Err(e) => {
            return Html(e.to_string());
        }
    };
    let mut ctx = Context::new();
    ctx.insert("tasks", &tasks);
    ctx.insert("current_filter", &query);
    ctx.insert("filter_value", &query.join(" "));
    Html(TEMPLATES.render("tasks.html", &ctx).unwrap())
}

async fn change_task_status(
    Form(mut multipart): Form<Params>
) -> Html<String> {
    if let Some(task) = multipart.task() {
        let mut tasks = list_tasks(multipart.query()).unwrap();
        if let Some(existing_task) = tasks.iter_mut().find(|v| v.uuid == task.uuid) {
            existing_task.status = task.status.clone();
            match update_tasks(existing_task) {
                Ok(_) => {
                    info!("Task was updated");
                }
                Err(e) => {
                    error!("Task was not updated: {}", e);
                }
            }
        }
    }
    get_tasks_view(multipart)
}