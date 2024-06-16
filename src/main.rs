use std::collections::HashMap;
use anyhow::Error;
use axum::{Form, Router, routing::get};
use axum::extract::{Multipart, Query};
use axum::response::Html;
use axum::routing::post;
use serde::{Deserialize, Serialize};
use tera::{Context, Tera};
use tracing::{debug, error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use org_me::endpoints::tasks::{list_tasks, Task, task_add, task_undo, task_undo_report, update_task_status};
use org_me::{FlashMsg, NewTask, TEMPLATES, TWGlobalState};
use org_me::endpoints::tasks::task_query_builder::TaskQuery;


#[tokio::main]
async fn main() {
    // initialize tracing

    init_tracing();

    // build our application with a route
    let app = Router::new()
        .route("/", get(front_page))
        .nest_service(
            "/dist",
            tower_http::services::ServeDir::new("./dist"),
        )
        .route("/tasks", get(tasks_display))
        .route("/tasks", post(change_task_status))
        .route("/tasks/undo/report", get(get_undo_report))
        .route("/tasks/undo/confirmed", post(undo_last_change))
        .route("/msg", get(display_flash_message))
        .route("/tasks/add", get(display_task_add_window))
        .route("/tasks/add", post(create_new_task))
        .route("/msg_clr", get(clear_flash_message))
        ;

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn init_tracing() {
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
}

async fn clear_flash_message() -> Html<String> {
    Html("".to_string())
}

async fn display_flash_message(Query(msg): Query<FlashMsg>) -> Html<String> {
    let mut ctx = Context::new();
    ctx.insert("msg", &msg.msg());
    ctx.insert("timeout", &msg.timeout());
    Html(TEMPLATES.render("flash_msg.html", &ctx).unwrap())
}

async fn get_undo_report() -> Html<String> {
    match task_undo_report() {
        Ok(s) => {
            let mut ctx = Context::new();
            let lines = s.lines();
            let first_line: String = lines.clone().take(1).collect();
            let mut rest_lines: Vec<String> = lines.skip(1).map(|v| v.to_string()).collect();
            rest_lines.pop();
            ctx.insert("heading", &first_line);
            ctx.insert("report", &rest_lines);
            Html(TEMPLATES.render("undo_report.html", &ctx).unwrap())
        }
        Err(e) => {
            let mut ctx = Context::new();
            ctx.insert("heading", &e.to_string());
            Html(TEMPLATES.render("error.html", &ctx).unwrap())
        }
    }
}

async fn display_task_add_window(Query(params): Query<TWGlobalState>) -> Html<String> {
    let tq: TaskQuery = params.filter_value().clone().and_then(|v| {
        if v == "" {
            Some(TaskQuery::default())
        } else {
            Some(serde_json::from_str(&v).unwrap_or(TaskQuery::default()))
        }
    }).or(Some(TaskQuery::default())).unwrap();
    let mut ctx = Context::new();
    ctx.insert("tags", &tq.tags().join(" "));
    ctx.insert("project", tq.project());
    Html(TEMPLATES.render("task_add.html", &ctx).unwrap())
}

async fn create_new_task(Form(new_task): Form<NewTask>) -> Html<String> {
    task_add(&new_task).unwrap();
    let mut ctx = Context::new();
    ctx.insert("has_toast", &true);
    ctx.insert("toast_msg", "New task created");
    ctx.insert("toast_timeout", &15);
    let s = if let Some(tw_q) = new_task.filter_value() {
        serde_json::from_str(tw_q).unwrap()
    } else {
        TaskQuery::default()
    };
    get_tasks_view(s, Some(ctx))
}


async fn undo_last_change(Query(params): Query<TWGlobalState>) -> Html<String> {
    task_undo().unwrap();
    let mut ctx = Context::new();
    ctx.insert("has_toast", &true);
    ctx.insert("toast_msg", "Undo successful");
    ctx.insert("toast_timeout", &15);
    get_tasks_view(org_me::task_query_previous_params(&params), Some(ctx))
}

async fn front_page() -> Html<String> {
    let tq = TaskQuery::new(TWGlobalState::default());
    let tasks = list_tasks(tq.clone()).unwrap();
    let task_list: Vec<Task> = tasks.values().cloned().collect();
    let mut ctx = Context::new();
    ctx.insert("tasks_db", &tasks);
    ctx.insert("tasks", &task_list);
    ctx.insert("current_filter", &tq.as_filter_text());
    ctx.insert("filter_value", &serde_json::to_string(&tq).unwrap());
    Html(TEMPLATES.render("base.html", &ctx).unwrap())
}

async fn tasks_display(Query(params): Query<TWGlobalState>) -> Html<String> {
    get_tasks_view(org_me::task_query_merge_previous_params(&params), None)
}

fn get_tasks_view(tq: TaskQuery, ctx: Option<Context>) -> Html<String> {
    let tasks = match list_tasks(tq.clone()) {
        Ok(t) => { t }
        Err(e) => {
            return Html(e.to_string());
        }
    };
    let task_list: Vec<Task> = tasks.values().cloned().collect();
    let mut ctx_b = if let Some(ctx) = ctx {
        ctx
    } else {
        Context::new()
    };
    ctx_b.insert("tasks_db", &tasks);
    ctx_b.insert("tasks", &task_list);
    ctx_b.insert("current_filter", &tq.as_filter_text());
    ctx_b.insert("filter_value", &serde_json::to_string(&tq).unwrap());
    Html(TEMPLATES.render("tasks.html", &ctx_b).unwrap())
}

async fn change_task_status(Form(multipart): Form<TWGlobalState>) -> Html<String> {
    if let Some(task) = org_me::from_task_to_task_update(&multipart) {
        match update_task_status(task) {
            Ok(_) => {
                info!("Task was updated");
            }
            Err(e) => {
                error!("Task was not updated: {}", e);
            }
        }
    }
    get_tasks_view(org_me::task_query_previous_params(&multipart), None)
}