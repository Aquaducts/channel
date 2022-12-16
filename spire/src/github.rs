use crate::models::AccessToken;
use anyhow::Result;

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

pub struct GithubApp {
    pub app_id: String,
    private_key_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    exp: i64,
    iat: i64,
}

impl GithubApp {
    pub fn new(app_id: String, private_key_path: String) -> Self {
        Self {
            app_id,
            private_key_path,
        }
    }

    pub fn generate_app_key(&self) -> String {
        let claims = Claims {
            iss: self.app_id.to_string(),
            iat: chrono::Utc::now().timestamp() - 60,
            exp: chrono::Utc::now().timestamp() + (10 * 60),
        };

        let key = std::fs::read_to_string(&self.private_key_path).unwrap();

        let header = Header {
            alg: Algorithm::RS256,
            ..Default::default()
        };
        encode(
            &header,
            &claims,
            &EncodingKey::from_rsa_pem(key.as_bytes()).unwrap(),
        )
        .unwrap()
    }

    pub async fn get_installation_access_token(
        &self,
        app_key: String,
        install_id: i64,
    ) -> Result<AccessToken> {
        let client = reqwest::Client::new();
        let access_token = client
            .post(format!(
                "https://api.github.com/app/installations/{}/access_tokens",
                install_id
            ))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", &"Bamboo CI".to_string())
            .header("Authorization", &format!("Bearer {app_key}"))
            .body(serde_json::to_string(&serde_json::json!({
                "permissions": {
                    "contents": "read"
                }
            }))?)
            .build()
            .unwrap();

        let res = client.execute(access_token).await?;
        Ok(serde_json::from_str::<AccessToken>(&res.text().await?)?)
    }
}
