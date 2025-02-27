use axum::extract::Query;
use axum::response::Html;
use axum::routing::post;
use axum::{routing::get, Form, Router};
use indexmap::IndexMap;
use rand::distr::{Alphanumeric, SampleString};
use std::collections::{HashMap, HashSet};
use std::env;
use taskwarrior_web::endpoints::tasks::task_query_builder::TaskQuery;
use taskwarrior_web::endpoints::tasks::{fetch_active_task, get_task_details, list_tasks, mark_task_as_done, run_annotate_command, run_denotate_command, run_modify_command, task_add, task_undo, task_undo_report, toggle_task_active, Task, TaskUUID, TAG_KEYWORDS};
use taskwarrior_web::{
    task_query_merge_previous_params, task_query_previous_params, FlashMsg, NewTask, TWGlobalState,
    TaskActions, TEMPLATES,
};
use tera::Context;
use tower::ServiceExt;
use tracing::{error, info, trace};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use taskwarrior_web::endpoints::tasks;

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
        .nest_service("/dist", tower_http::services::ServeDir::new("./dist"))
        .route("/tasks", get(tasks_display))
        .route("/tasks", post(do_task_actions))
        .route("/tasks/undo/report", get(get_undo_report))
        .route("/tasks/undo/confirmed", post(undo_last_change))
        .route("/msg", get(display_flash_message))
        .route("/tasks/add", get(display_task_add_window))
        .route("/tasks/active", get(get_active_task))
        .route("/tasks/add", post(create_new_task))
        .route("/msg_clr", get(just_empty))
        .route("/tag_bar", get(get_tag_bar))
        .route("/task_action_bar", get(get_task_action_bar))
        .route("/task_details", get(display_task_details))
        .route("/bars", get(get_bar));

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
        .with(tracing_subscriber::fmt::layer().with_line_number(true))
        .init();
}

async fn display_task_details(Query(param): Query<HashMap<String, String>>) -> Html<String> {
    let task_id = param.get("task_id").unwrap().clone();
    let task = get_task_details(task_id).unwrap();
    let tq = TaskQuery::new(TWGlobalState::default());
    let tasks = list_tasks(&tq).unwrap();
    let mut ctx = Context::new();
    ctx.insert("tasks_db", &tasks);
    ctx.insert("task", &task);
    Html(TEMPLATES.render("task_details.html", &ctx).unwrap())
}

async fn get_active_task() -> Html<String> {
    let mut ctx = Context::new();
    if let Ok(v) = fetch_active_task() {
        if let Some(v) = v {
            ctx.insert("active_task", &v);
        }
    }
    Html(TEMPLATES.render("active_task.html", &ctx).unwrap())
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
    let tq: TaskQuery = params
        .filter_value()
        .clone()
        .and_then(|v| {
            if v == "" {
                Some(TaskQuery::default())
            } else {
                Some(serde_json::from_str(&v).unwrap_or(TaskQuery::default()))
            }
        })
        .or(Some(TaskQuery::default()))
        .unwrap();
    let mut ctx = Context::new();
    ctx.insert("tags", &tq.tags().join(" "));
    ctx.insert("project", tq.project());
    Html(TEMPLATES.render("task_add.html", &ctx).unwrap())
}

async fn undo_last_change(Query(params): Query<TWGlobalState>) -> Html<String> {
    task_undo().unwrap();
    let fm = FlashMsg::new("Undo successful", None);
    get_tasks_view(task_query_previous_params(&params), Some(fm))
}

fn make_shortcut(name: &str, shortcuts: &mut HashSet<String>) -> String {
    let alpha = Alphanumeric::default();
    let mut len = 2;
    let mut tries = 0;
    loop {
        let shortcut = alpha.sample_string(&mut rand::rng(), 2).to_lowercase();
        if !shortcuts.contains(&shortcut) {
            shortcuts.insert(shortcut.clone());
            return shortcut;
        }
        tries += 1;
        if tries > 700 {
            len += 1;
            if len > 3 {
                panic!("too many shortcuts! this should not happen");
            }
            tries = 0;
        }
    }
}

struct TaskViewDataRetType {
    tasks: IndexMap<TaskUUID, Task>,
    tag_map: HashMap<String, String>,
    shortcuts: HashSet<String>,
    task_list: Vec<Task>,
}

fn get_tasks_view_data(
    mut tasks: IndexMap<TaskUUID, Task>,
    filters: &Vec<String>,
) -> TaskViewDataRetType {
    let mut tag_map: HashMap<String, String> = HashMap::new();
    let mut shortcuts = HashSet::new();
    let task_list: Vec<Task> = tasks
        .values_mut()
        .map(|task| {
            if let Some(tags) = &mut task.tags {
                tags.iter_mut().for_each(|v| {
                    if !tasks::is_tag_keyword(v) {
                        *v = format!("+{}", v);
                    }
                    let shortcut = make_shortcut(&v, &mut shortcuts);
                    tag_map.insert(v.clone(), shortcut);
                });
            }
            if let Some(project) = &task.project {
                // the project is not in the map, so all of it can be added
                if let None = tag_map.get(project) {
                    let parts: Vec<_> = project.split('.').collect();
                    let mut total_parts = vec![];
                    for part in parts {
                        total_parts.push(part);
                        let s = total_parts.join(".");
                        let shortcut = make_shortcut(&s, &mut shortcuts);
                        tag_map.insert(s, shortcut);
                    }
                }
            }
            task.clone()
        })
        .collect();
    for filter in filters {
        if !tag_map.contains_key(filter) {
            if tasks::is_tag_keyword(filter) {

            }
            else if tasks::is_a_tag(filter) {
                let ky = format!("@{}", filter);
                let shortcut = make_shortcut(&ky, &mut shortcuts);
                tag_map.insert(ky, shortcut);
            } else {
                let parts: Vec<_> = filter.split('.').collect();
                let mut total_parts = vec![];
                for part in parts {
                    total_parts.push(part);
                    let ky = format!("@{}", filter);
                    let s = total_parts.join(".");
                    let shortcut = make_shortcut(&ky, &mut shortcuts);
                    tag_map.insert(ky, shortcut);
                }
            }
        }
    }

    TaskViewDataRetType {
        tasks,
        task_list,
        shortcuts,
        tag_map,
    }
}

async fn front_page() -> Html<String> {
    let tq = TaskQuery::new(TWGlobalState::default());
    let tasks = list_tasks(&tq).unwrap();
    let filters = tq.as_filter_text();
    let TaskViewDataRetType {
        tasks,
        tag_map,
        shortcuts,
        task_list,
    } = get_tasks_view_data(tasks, &filters);
    let mut ctx = Context::new();
    ctx.insert("tasks_db", &tasks);
    ctx.insert("tasks", &task_list);
    ctx.insert("current_filter", &tq.as_filter_text());
    ctx.insert("filter_value", &serde_json::to_string(&tq).unwrap());
    ctx.insert("tags_map", &tag_map);
    let n = env::var("DISPLAY_TIME_OF_THE_DAY")
        .unwrap_or("0".to_string())
        .parse::<i32>()
        .unwrap_or(0);
    ctx.insert("display_time_of_the_day", &n);
    let t = tasks.iter().find(|(_, task)| task.start.is_some());
    println!("{:?} {:?}", tag_map, shortcuts);
    if let Some((_, v)) = t {
        ctx.insert("active_task", v);
    }
    Html(TEMPLATES.render("base.html", &ctx).unwrap())
}

async fn tasks_display(Query(params): Query<TWGlobalState>) -> Html<String> {
    get_tasks_view(task_query_merge_previous_params(&params), None)
}

fn get_tasks_view(tq: TaskQuery, flash_msg: Option<FlashMsg>) -> Html<String> {
    dotenvy::dotenv().unwrap();
    let tasks = match list_tasks(&tq) {
        Ok(t) => t,
        Err(e) => {
            return Html(e.to_string());
        }
    };
    let current_filter = tq.as_filter_text();
    let TaskViewDataRetType {
        tasks,
        tag_map,
        shortcuts,
        task_list,
    } = get_tasks_view_data(tasks, &current_filter);
    let mut filter_ar = vec![];
    for filter in current_filter.iter() {
        if filter.starts_with("project:") {
            let mut stack = vec![];
            for part in filter.split(":").nth(1).unwrap().split(".") {
                stack.push(part);
                filter_ar.push(format!("project:{}", stack.join(".")))
            }
        } else {
            filter_ar.push(filter.to_string());
        }
    }
    trace!("{:?}", current_filter);
    let mut ctx_b = Context::new();
    ctx_b.insert("tasks_db", &tasks);
    ctx_b.insert("tasks", &task_list);
    ctx_b.insert("current_filter", &filter_ar);
    ctx_b.insert("filter_value", &serde_json::to_string(&tq).unwrap());
    ctx_b.insert("tags_map", &tag_map);
    let n = env::var("DISPLAY_TIME_OF_THE_DAY")
        .unwrap_or("0".to_string())
        .parse::<i32>()
        .unwrap_or(0);
    ctx_b.insert("display_time_of_the_day", &n);
    if let Some(msg) = flash_msg {
        ctx_b.insert("has_toast", &true);
        ctx_b.insert("toast_msg", msg.msg());
        ctx_b.insert("toast_timeout", &msg.timeout());
    }
    let t = tasks.iter().find(|(_, task)| task.start.is_some());
    if let Some((_, v)) = t {
        ctx_b.insert("active_task", v);
    }
    Html(TEMPLATES.render("tasks.html", &ctx_b).unwrap())
}

async fn create_new_task(Form(new_task): Form<NewTask>) -> Html<String> {
    let fm = match task_add(&new_task) {
        Ok(_) => FlashMsg::new("New task created", None),
        Err(e) => FlashMsg::new(&format!("Failed to create new task: {e}"), None),
    };
    let s = if let Some(tw_q) = new_task.filter_value() {
        serde_json::from_str(tw_q).unwrap()
    } else {
        TaskQuery::default()
    };
    get_tasks_view(s, Some(fm))
}

async fn do_task_actions(Form(multipart): Form<TWGlobalState>) -> Html<String> {
    info!("{:?}", multipart);
    let fm = match multipart.action().clone().unwrap() {
        TaskActions::StatusUpdate => {
            if let Some(task) = taskwarrior_web::from_task_to_task_update(&multipart) {
                match mark_task_as_done(task.clone()) {
                    Ok(_) => FlashMsg::new(&format!("Task [{}] was updated", task.uuid), None),
                    Err(e) => {
                        error!("Failed: {}", e);
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
                Ok(v) => {
                    if v {
                        FlashMsg::new(
                            &format!(
                                "Task {} started, any other tasks running were stopped",
                                task_uuid
                            ),
                            None,
                        )
                    } else {
                        FlashMsg::new(&format!("Task {} stopped", task_uuid), None)
                    }
                }
                Err(e) => {
                    error!("Failed: {}", e);
                    FlashMsg::new(&format!("Failed to update task: {e}"), None)
                }
            }
        }
        TaskActions::ModifyTask => {
            let cmd = multipart.task_entry().clone().unwrap();
            if cmd.is_empty() {
                error!("Failed: No annotation provided");
                FlashMsg::new("Failed to execute command, none provided", None)
            } else {
                match run_modify_command(multipart.uuid().as_ref().unwrap(), &cmd) {
                    Ok(_) => FlashMsg::new("Modify command success", None),
                    Err(e) => FlashMsg::new(&format!("Modify command failed: {}", e), None),
                }
            }
        }
        TaskActions::AnnotateTask => {
            let cmd = multipart.task_entry().clone().unwrap();
            if cmd.is_empty() {
                error!("Failed: No command provided");
                FlashMsg::new("Failed to execute command, none provided", None)
            } else {
                match run_annotate_command(multipart.uuid().as_ref().unwrap(), &cmd) {
                    Ok(_) => FlashMsg::new("Annotation added", None),
                    Err(e) => FlashMsg::new(&format!("Annotation command failed: {}", e), None),
                }
            }
        }
        TaskActions::DenotateTask => {
            match run_denotate_command(multipart.uuid().as_ref().unwrap()) {
                Ok(_) => FlashMsg::new("Denotated task", None),
                Err(e) => FlashMsg::new(&format!("Denotation command failed: {}", e), None),
            }
        }
    };
    get_tasks_view(task_query_previous_params(&multipart), Some(fm))
}
