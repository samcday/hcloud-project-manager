use std::collections::HashMap;

use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, Arg, SubCommand,
};
use eyre::{eyre, Result, WrapErr};
use reqwest::header;
use reqwest::redirect::Policy;
use select::document::Document;
use select::predicate::Attr;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing_subscriber::EnvFilter;

const PER_PAGE: &str = "25";
const LOGIN_URL: &str = "https://accounts.hetzner.com/login";
const LOGIN_CHECK_URL: &str = "https://accounts.hetzner.com/login_check";
const AUTHORIZE_URL: &str = "https://accounts.hetzner.com/oauth/authorize";
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
                .about("Get project ID by name")
                .arg(
                    Arg::from_usage("-t, --token=<TOKEN> 'Hetzner API user token'")
                        .env("HCLOUD_USER_TOKEN")
                        .required(true),
                )
                .arg(Arg::from_usage("<name> 'Name of project'")),
        )
        .subcommand(
            SubCommand::with_name("token")
                .about("Generate API token for a project")
                .arg(
                    Arg::from_usage("-t, --token=<TOKEN> 'Hetzner API user token'")
                        .env("HCLOUD_USER_TOKEN")
                        .required(true),
                )
                .arg(Arg::from_usage("<name> 'Name of project'")),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("login") {
        let username = matches.value_of("username").unwrap();
        let password = matches.value_of("password").unwrap();

        let user_token = get_user_token(&username, &password).await?;
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

        let project_id = response.json::<Value>().await?["project"]["id"]
            .as_i64()
            .unwrap();
        println!("{}", project_id);
    }

    if let Some(matches) = matches.subcommand_matches("id") {
        println!(
            "{}",
            get_project_id(
                matches.value_of("token").unwrap(),
                matches.value_of("name").unwrap(),
            )
            .await?
        );
    }

    if let Some(matches) = matches.subcommand_matches("token") {
        let user_token = matches.value_of("token").unwrap();
        let project_id = get_project_id(user_token, matches.value_of("name").unwrap()).await?;

        let client = reqwest::Client::new();
        let response = client
            .post("https://api.hetzner.cloud/v1/_tokens")
            .json(&json!({ "type": "project_user", "project": project_id.to_string() }))
            .bearer_auth(user_token)
            .send()
            .await?;

        println!(
            "{}",
            response.json::<Value>().await?["secret_token"]
                .as_str()
                .unwrap()
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

// Performs a login to Hetzner account service and obtains a user-level API token for Hetzner Cloud.
async fn get_user_token(username: &str, password: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .redirect(Policy::custom(|attempt| {
            // We don't follow too many redirects, or any redirects to the Cloud Console.
            if attempt.previous().len() > 3 || attempt.url().to_string().starts_with(CONSOLE_URL) {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION"),
        ))
        .build()?;

    // This is the request that gives us a valid user API token for Hetzner Cloud.
    // We hit it twice, first time to prime a PHPSESSID and get redirected to login page, then again
    // once we've performed a login.
    let authorize_req = client
        .get(AUTHORIZE_URL)
        .query(&[
            ("response_type", "id_token token"),
            ("client_id", "cloud_421"),
            ("state", "spaghetti"),
            ("nonce", "potato"),
            ("redirect_uri", "https://console.hetzner.cloud/"),
            ("scope", "openid"),
        ])
        .build()?;

    let login_resp = client.execute(authorize_req).await?;
    if login_resp.url().as_str() != LOGIN_URL {
        return Err(eyre!("expected login page, got {}", login_resp.url()));
    }

    // Grab a CSRF token from the login page. We'll also pickup a PHPSESSID here.
    let login_page = Document::from(
        login_resp
            .text()
            .await
            .wrap_err("Load login page failed")?
            .as_str(),
    );
    let csrf_token = login_page
        .find(Attr("name", "_csrf_token"))
        .next()
        .and_then(|t| t.attr("value"))
        .ok_or(eyre!("CSRF token not found"))?;

    // Now we can perform a login.
    let login_response = client
        .post(LOGIN_CHECK_URL)
        .form(&[
            ("_username", username),
            ("_password", password),
            ("_csrf_token", csrf_token),
        ])
        .send()
        .await?;

    // If we were redirected to the login page, it probably means invalid username/password.
    if login_response.url().as_str() == LOGIN_URL {
        return Err(eyre!("login failed, check username/password"));
    }

    let location = login_response
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(eyre!("no Location header"))?;
    if !location.starts_with(CONSOLE_URL) {
        return Err(eyre!(
            "expected redirect to Cloud Console, got {}",
            location
        ));
    }

    let location = url::Url::parse(location)?;
    let fragment = location
        .fragment()
        .ok_or(eyre!("oauth fragment missing"))?
        .clone();

    let oauth_data = url::form_urlencoded::parse(fragment.as_bytes())
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect::<HashMap<String, String>>();

    let token_response = client
        .post("https://api.hetzner.cloud/v1/_tokens")
        .json(&json!({
            "access_token": oauth_data.get("access_token").ok_or(eyre!("access_token missing"))?,
            "id_token": oauth_data.get("id_token").ok_or(eyre!("id_token missing"))?,
            "type": "user",
        }))
        .send()
        .await?;

    Ok(token_response
        .json::<Value>()
        .await
        .wrap_err("failed to parse token response")?
        .get("secret_token")
        .and_then(|v| v.as_str())
        .ok_or(eyre!("secret_token missing"))?
        .to_owned())
}
