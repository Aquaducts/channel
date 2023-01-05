use actix_web::{
    get,
    web::{self},
    Error, HttpRequest, HttpResponse,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{config::CONFIG, github::GithubApp, Spire};

// TODO: move all request models to one file
#[derive(Debug, Serialize, Deserialize)]
pub struct NewInstall {
    pub installation_id: i64,
    pub setup_action: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    exp: i64,
    iat: i64,
}

#[get("/new_install")]
async fn manage_new_install(
    app: web::Data<Spire>,
    params: web::Query<NewInstall>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let gh_config = CONFIG.github.clone();
    let gh_app = GithubApp::new(gh_config.app_id, gh_config.key_path);

    let access_token = gh_app
        .get_installation_access_token(gh_app.generate_app_key(), params.0.installation_id)
        .await
        .unwrap();

    let client = reqwest::Client::new();

    sqlx::query::<_>(r#"INSERT INTO installations(id) VALUES($1)"#)
        .bind(params.0.installation_id)
        .execute(&app.database.0)
        .await
        .unwrap();

    let repos = client
        .get("https://api.github.com/installation/repositories")
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", &"Bamboo CI".to_string())
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Authorization", &format!("Bearer {}", access_token.token))
        .build()
        .unwrap();

    #[derive(Debug, Serialize, Deserialize)]
    pub struct RepoOwner {
        pub login: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct RRepos {
        pub id: i64,
        pub name: String,
        pub owner: RepoOwner,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct InstallRepos {
        pub repositories: Vec<RRepos>,
    }

    if let Ok(res) = client.execute(repos).await {
        let repos = serde_json::from_str::<InstallRepos>(&res.text().await.unwrap()).unwrap();
        let install_id = &params.0.installation_id;
        let install_query_values = repos
            .repositories
            .into_iter()
            .map(|e| format!("({},'{}','{}',{})", e.id, e.name, e.owner.login, install_id))
            .collect::<Vec<String>>()
            .join(",");

        sqlx::query::<_>(&format!(
            "INSERT INTO repos(gh_id, name, owner, install) VALUES {}",
            install_query_values
        ))
        .execute(&app.database.0)
        .await
        .unwrap();
    };

    Ok(HttpResponse::PermanentRedirect()
        .append_header((
            "Location",
            format!(
                "https://ci.yiff.day/get_repos/{}",
                &params.0.installation_id
            )
            .as_str(),
        ))
        .finish())
}
