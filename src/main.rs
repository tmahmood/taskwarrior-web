use std::collections::HashMap;
use axum::{Router, routing::get};
use axum::extract::Query;
use axum::response::Html;
use tera::{Context, Tera};
use tracing::debug;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use org_me::endpoints::tasks::list_tasks;
use org_me::Params;

lazy_static::lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = match Tera::new("templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera.register_function("project_name", get_project_name_link());
        tera.autoescape_on(vec![
            ".html",
            ".sql"
        ]);
        tera
    };
}

fn get_project_name_link() -> impl tera::Function {
    Box::new(move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
        let r = String::new();
        let pname = tera::from_value::<String>(
            args.get("full_name").clone().unwrap().clone()
        ).unwrap();
        let index = tera::from_value::<usize>(
            args.get("index").clone().unwrap().clone()
        ).unwrap();
        let r:Vec<&str> = pname.split(".").take(index).collect();
        Ok(tera::to_value(r.join(".")).unwrap())
    })
}
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
            tower_http::services::ServeDir::new("./dist")
        )
        .route("/tasks", get(tasks_display))
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
    let query = params.query();
    let tasks = match list_tasks(query.clone()) {
        Ok(t) => {t},
        Err(e) => {
            return Html(e.to_string())
        }
    };
    let mut ctx = Context::new();
    ctx.insert("tasks", &tasks);
    ctx.insert("current_filter", &query);
    ctx.insert("filter_value", &query.join(" "));

    Html(TEMPLATES.render("tasks.html", &ctx).unwrap())
}
