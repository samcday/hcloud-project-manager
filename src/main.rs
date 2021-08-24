use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, Arg, SubCommand,
};
use eyre::{eyre, Result, WrapErr};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use tracing_subscriber::EnvFilter;

use reqwest::header;
use reqwest::header::HeaderMap;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

const PER_PAGE: &str = "25";
const CONSOLE_URL: &str = "https://console.hetzner.cloud";

#[derive(Debug, Deserialize)]
struct Project {
    id: u32,
    name: String,
    usage_alert_threshold: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Pagination {
    page: u32,
    per_page: u32,
    next_page: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct Meta {
    pagination: Pagination,
}

#[derive(Deserialize, Debug)]
struct ProjectListResponse {
    projects: Vec<Project>,
    meta: Meta,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let matches = app_from_crate!()
        .subcommand(
            SubCommand::with_name("login")
                .about("Generate a user API token from credentials")
                .arg(
                    Arg::from_usage("-u, --username=<USER> 'Hetzner account username'")
                        .env("HETZNER_USERNAME")
                        .required(true),
                )
                .arg(
                    Arg::from_usage("-p, --password=<PASS> 'Hetzner account password'")
                        .env("HETZNER_PASSWORD")
                        .required(true),
                )
                .arg(
                    Arg::from_usage("--headless-no-sandbox 'Disable Headless Chrome sandbox'")
                        .env("HEADLESS_NO_SANDBOX"),
                )
                .arg(
                    Arg::from_usage("--headless-path=[PATH] 'Path to Chrome binary'")
                        .env("HEADLESS_PATH"),
                ),
        )
        .subcommand(
            SubCommand::with_name("create")
                .about("Create a new project")
                .arg(
                    Arg::from_usage("-t, --token=<TOKEN> 'Hetzner API user token'")
                        .env("HCLOUD_USER_TOKEN")
                        .required(true),
                )
                .arg_from_usage("<name> 'Name of new project'"),
        )
        .subcommand(
            SubCommand::with_name("delete")
                .about("Delete a project")
                .arg(
                    Arg::from_usage("-t, --token=<TOKEN> 'Hetzner API user token'")
                        .env("HCLOUD_USER_TOKEN")
                        .required(true),
                )
                .arg_from_usage("<name> 'Name of project to delete'"),
        )
        .subcommand(
            SubCommand::with_name("id")
                .arg(
                    Arg::from_usage("-t, --token=<TOKEN> 'Hetzner API user token'")
                        .env("HCLOUD_USER_TOKEN")
                        .required(true),
                )
                .about("Get project ID by name")
                .arg(Arg::from_usage("<name> 'Name of project'")),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("login") {
        let username = matches.value_of("username").unwrap();
        let password = matches.value_of("password").unwrap();

        let browser = Browser::new(
            LaunchOptionsBuilder::default()
                .path(matches.value_of("headless-path").map(|p| p.into()))
                .sandbox(!matches.is_present("headless-no-sandbox"))
                .build()
                .map_err(|e| eyre!("error building headless_chrome::LaunchOptions: {}", e))?,
        )
        .map_err(|e| e.compat())
        .wrap_err("error initializing Headless Chrome")?;

        let user_token = get_user_token(&username, &password, browser).map_err(|e| e.compat())?;
        println!("{}", user_token);
    }

    if let Some(matches) = matches.subcommand_matches("create") {
        let user_token = matches.value_of("token").unwrap();
        let project_name = matches.value_of("name").unwrap();

        let client = reqwest::Client::new();
        let response = client
            .post("https://api.hetzner.cloud/v1/_projects")
            .header(header::CONTENT_TYPE, "application/json")
            .body(json!({ "name": project_name }).to_string())
            .bearer_auth(user_token)
            .send()
            .await?;

        println!("{}", response.text().await?);
    }

    if let Some(matches) = matches.subcommand_matches("id") {
        println!(
            "{}",
            get_project_id(
                matches.value_of("token").unwrap(),
                matches.value_of("name").unwrap()
            )
            .await?
        );
    }

    if let Some(matches) = matches.subcommand_matches("delete") {
        let user_token = matches.value_of("token").unwrap();
        let id = get_project_id(user_token, matches.value_of("name").unwrap()).await?;

        reqwest::Client::new()
            .delete(format!("https://api.hetzner.cloud/v1/_projects/{}", id).as_str())
            .bearer_auth(user_token)
            .send()
            .await?;
    }

    Ok(())
}

async fn get_project_id(token: &str, project_name: &str) -> Result<u32> {
    let mut next_page = Some(1);
    while let Some(page) = next_page {
        let resp: ProjectListResponse = reqwest::Client::new()
            .get("https://api.hetzner.cloud/v1/_projects")
            .bearer_auth(token)
            .query(&[("per_page", PER_PAGE), ("page", &page.to_string())])
            .send()
            .await?
            .json()
            .await?;

        for proj in resp.projects {
            if proj.name == project_name {
                return Ok(proj.id);
            }
        }

        next_page = resp.meta.pagination.next_page;
    }

    Err(eyre!("project {} not found", project_name))
}

// Uses a headless browser and provided login credentials to obtain a Hetzner API user token.
// This token is permitted to create/delete projects, and obtain API project_user tokens.
fn get_user_token(
    username: &str,
    password: &str,
    browser: Browser,
) -> Result<String, failure::Error> {
    let tab = browser.wait_for_initial_tab()?;
    tab.navigate_to(CONSOLE_URL)?;
    tab.wait_until_navigated()?;
    if !tab
        .get_url()
        .starts_with("https://accounts.hetzner.com/login")
    {
        return Err(failure::format_err!(
            "expected navigation to login page, got {} instead",
            tab.get_url()
        ));
    }

    tab.wait_for_element("#_username")?.type_into(username)?;
    tab.wait_for_element("#_password")?.type_into(password)?;
    tab.wait_for_element("#submit-login")?.click()?;

    tab.wait_for_element(".user-details__robotcn")?;

    // Scoop the token storage out of the "tokens" cookie saved by console.hetzner.cloud.
    let raw_token = tab
        .get_cookies()?
        .iter()
        .find(|cookie| cookie.name == "tokens" && cookie.domain == "console.hetzner.cloud")
        .ok_or(failure::format_err!("Couldn't find tokens cookie"))?
        .value
        .to_owned();

    // The access token is wrapped in url-encoded JSON. So we go diggin'.
    Ok(serde_json::from_str::<HashMap<String, serde_json::Value>>(
        form_urlencoded::parse(("v=".to_owned() + &raw_token).as_bytes())
            .next()
            .ok_or_else(|| failure::format_err!("couldn't parse url-encoded token"))?
            .1
            .as_ref(),
    )?
    .values()
    .next()
    .and_then(|v| v.get("token"))
    .and_then(Value::as_str)
    .ok_or_else(|| failure::format_err!("couldn't locate token in cookie"))?
    .to_owned())
}
