use tracing_subscriber::EnvFilter;

const CONSOLE_URL: &str = "https://console.hetzner.cloud";

use serde::{Serialize, Deserialize};
use eyre::{eyre, Result};
use headless_chrome::{Browser};
use std::collections::HashMap;
use clap::{crate_version, App};

#[derive(Serialize, Deserialize, Debug)]
struct CloudConsoleToken {
    token: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::fmt().with_env_filter(EnvFilter::from_default_env()).init();
    //
    // let matches = App::new("hcloud-project-manager")
    //     .version(crate_version!())
    //     .subcommand()
    //     .get_matches();

    let username = std::env::var("HCLOUD_USERNAME")?;
    let password = std::env::var("HCLOUD_PASSWORD")?;

    let rawToken = get_token(&username, &password).map_err(|e| e.compat())?;

    let tokenStr = form_urlencoded::parse(("v=".to_owned()+&rawToken).as_bytes())
        .next().ok_or_else(|| eyre!("couldn't parse url-encoded token"))?.1.into_owned();

    let token = serde_json::from_str::<HashMap<String, CloudConsoleToken>>(&tokenStr)?
        .values().next().ok_or_else(|| eyre!("couldn't locate token in cookie"))?.token.to_owned();

    let client = reqwest::Client::new();
    let response = client.get("https://api.hetzner.cloud/v1/_projects")
        .bearer_auth(token)
        .send().await?;

    println!("{}", response.text().await?);

    Ok(())
}

fn get_token(username: &str, password: &str) -> Result<String, failure::Error> {
    let browser = Browser::default()?;
    let tab = browser.wait_for_initial_tab()?;
    tab.navigate_to(CONSOLE_URL)?;
    tab.wait_until_navigated()?;
    if !tab.get_url().starts_with("https://accounts.hetzner.com/login") {
        return Err(failure::format_err!("onoes"));
    }

    tab.wait_for_element("#_username")?.type_into(username)?;
    tab.wait_for_element("#_password")?.type_into(password)?;
    tab.wait_for_element("#submit-login")?.click()?;

    tab.wait_for_element(".user-details__robotcn")?;
    println!("{}", tab.get_url());

    Ok(tab.get_cookies()?.iter()
        .find(|cookie| cookie.name == "tokens" && cookie.domain == "console.hetzner.cloud")
        .ok_or(failure::format_err!("Couldn't find tokens cookie"))?
        .value.to_owned()
    )

    // for cookie in tab.get_cookies()? {
    //     println!("{:?}", cookie);
    // }
    //
    // let png = tab.capture_screenshot(ScreenshotFormat::PNG, None, true)?;
    // let mut f = File::create("/tmp/test.png")?;
    // f.write_all(&png)?;
    //
    // Ok("hi mom".to_string())
}