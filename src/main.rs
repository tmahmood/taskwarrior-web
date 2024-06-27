use std::collections::HashMap;
use std::env;
use axum::{Form, Router, routing::get};
use axum::extract::Query;
use axum::response::Html;
use axum::routing::post;
use tera::Context;
use tracing::{info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use taskwarrior_web::endpoints::tasks::{get_task_details, list_tasks, Task, task_add, task_undo, task_undo_report, mark_task_as_done, toggle_task_active};
use taskwarrior_web::{FlashMsg, NewTask, task_query_merge_previous_params, task_query_previous_params, TaskActions, TEMPLATES, TWGlobalState};
use taskwarrior_web::endpoints::tasks::task_query_builder::TaskQuery;


#[tokio::main]
async fn main() {
    // initialize tracing
    init_tracing();
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(_) => {}
    };

    let addr = format!(
        "0.0.0.0:{}",
        env::var("TWK_SERVER_PORT").unwrap_or("3000".to_string())
    );

    // build our application with a route
    let app = Router::new()
        .route("/", get(front_page))
        .nest_service(
            "/dist",
            tower_http::services::ServeDir::new("./dist"),
        )
        .route("/tasks", get(tasks_display))
        .route("/tasks", post(do_task_actions))
        .route("/tasks/undo/report", get(get_undo_report))
        .route("/tasks/undo/confirmed", post(undo_last_change))
        .route("/msg", get(display_flash_message))
        .route("/tasks/add", get(display_task_add_window))
        .route("/tasks/add", post(create_new_task))
        .route("/msg_clr", get(just_empty))
        .route("/tag_bar", get(get_tag_bar))
        .route("/task_action_bar", get(get_task_action_bar))
        .route("/task_details", get(display_task_details))
        .route("/bars", get(get_bar))
        ;

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("running on address: {}", addr);
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

async fn display_task_details(Query(param): Query<HashMap<String, String>>) -> Html<String> {
    let task_id = param.get("task_id").unwrap().clone();
    let task = get_task_details(task_id).unwrap();
    let tq = TaskQuery::new(TWGlobalState::default());
    let tasks = list_tasks(tq.clone()).unwrap();
    let mut ctx = Context::new();
    ctx.insert("tasks_db", &tasks);
    ctx.insert("task", &task);
    Html(TEMPLATES.render("task_details.html", &ctx).unwrap())
}

async fn get_task_action_bar() -> Html<String> {
    let ctx = Context::new();
    Html(TEMPLATES.render("task_action_bar.html", &ctx).unwrap())
}

async fn get_bar(Query(param): Query<HashMap<String, String>>) -> Html<String> {
    if let Some(bar) = param.get("bar") {
        let ctx = Context::new();
        if bar == "left_action_bar" {
            Html(TEMPLATES.render("left_action_bar.html", &ctx).unwrap())
        } else {
            Html(TEMPLATES.render("task_action_bar.html", &ctx).unwrap())
        }
    } else {
        Html("".to_string())
    }
}


async fn get_tag_bar() -> Html<String> {
    let ctx = Context::new();
    Html(TEMPLATES.render("tag_bar.html", &ctx).unwrap())
}


async fn just_empty() -> Html<String> {
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
    let fm = match task_add(&new_task) {
        Ok(_) => FlashMsg::new("New task created", None),
        Err(e) => FlashMsg::new(&format!("Failed to create new task: {e}"), None)
    };
    let s = if let Some(tw_q) = new_task.filter_value() {
        serde_json::from_str(tw_q).unwrap()
    } else {
        TaskQuery::default()
    };
    get_tasks_view(s, Some(fm))
}


async fn undo_last_change(Query(params): Query<TWGlobalState>) -> Html<String> {
    task_undo().unwrap();
    let fm = FlashMsg::new("Undo successful", None);
    get_tasks_view(task_query_previous_params(&params), Some(fm))
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
    get_tasks_view(task_query_merge_previous_params(&params), None)
}

fn get_tasks_view(tq: TaskQuery, flash_msg: Option<FlashMsg>) -> Html<String> {
    let tasks = match list_tasks(tq.clone()) {
        Ok(t) => { t }
        Err(e) => {
            return Html(e.to_string());
        }
    };
    let task_list: Vec<Task> = tasks.values().cloned().collect();
    let mut ctx_b = Context::new();
    ctx_b.insert("tasks_db", &tasks);
    ctx_b.insert("tasks", &task_list);
    ctx_b.insert("current_filter", &tq.as_filter_text());
    ctx_b.insert("filter_value", &serde_json::to_string(&tq).unwrap());
    if let Some(msg) = flash_msg {
        ctx_b.insert("has_toast", &true);
        ctx_b.insert("toast_msg", msg.msg());
        ctx_b.insert("toast_timeout", &msg.timeout());
    }
    Html(TEMPLATES.render("tasks.html", &ctx_b).unwrap())
}


async fn do_task_actions(Form(multipart): Form<TWGlobalState>) -> Html<String> {
    let fm = match multipart.action().clone().unwrap() {
        TaskActions::StatusUpdate => {
            if let Some(task) = taskwarrior_web::from_task_to_task_update(&multipart) {
                match mark_task_as_done(task) {
                    Ok(_) => {
                        info!("Task was updated");
                        FlashMsg::new("Task was updated", None)
                    }
                    Err(e) => {
                        FlashMsg::new(&format!("Failed to update task: {e}"), None)
                    }
                }
            } else {
                FlashMsg::new("No task to update", None)
            }
        }
        TaskActions::ToggleTimer => {
            let task_uuid = multipart.uuid().clone().unwrap();
            match toggle_task_active(&task_uuid) {
                Ok(_) => {
                    info!("Task was updated");
                    FlashMsg::new("Task was updated", None)
                }
                Err(e) => {
                    FlashMsg::new(&format!("Failed to update task: {e}"), None)
                }
            }
        }
    };
    get_tasks_view(
        task_query_previous_params(&multipart),
        Some(fm)
    )
}

