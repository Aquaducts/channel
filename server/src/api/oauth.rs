use actix_web::{
    get,
    web::{self},
    Error, HttpRequest, HttpResponse, Responder,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{config::CONFIG, github::GithubApp, Spire};

#[derive(Serialize, Deserialize)]
pub struct LoginQuery {
    pub service: String,
}
// https://github.com/login/oauth/authorize?client_id=b7b8339cdba69a1df77b&scope=repo,user:email,read:user&state=schaHSGCjhasgcnbGSChasc
#[get("/login")]
async fn login(
    app: web::Data<Spire>,
    query: web::Query<LoginQuery>,
    _req: HttpRequest,
) -> Result<impl Responder, Error> {
    // ... generate all required data before sending back to github oauth screen
    Ok(HttpResponse::Ok().finish())
}
