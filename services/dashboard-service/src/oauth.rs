use anyhow::Result;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub tenant_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub token_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthState {
    pub csrf_token: String,
    pub pkce_verifier: String,
}

#[derive(Clone)]
pub struct OAuthManager {
    client: BasicClient,
    config: OAuthConfig,
    http_client: Client,
}

impl OAuthManager {
    pub fn new(config: OAuthConfig) -> Result<Self> {
        let auth_url = AuthUrl::new(format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/authorize",
            config.tenant_id
        ))?;

        let token_url = TokenUrl::new(format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            config.tenant_id
        ))?;

        let client = BasicClient::new(
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(RedirectUrl::new(config.redirect_uri.clone())?);

        Ok(Self {
            client,
            config,
            http_client: Client::new(),
        })
    }

    pub fn get_authorization_url(&self) -> Result<(Url, AuthState)> {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("https://graph.microsoft.com/Mail.Read".to_string()))
            .add_scope(Scope::new("https://graph.microsoft.com/User.Read".to_string()))
            .add_scope(Scope::new("offline_access".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        let auth_state = AuthState {
            csrf_token: csrf_token.secret().clone(),
            pkce_verifier: pkce_verifier.secret().clone(),
        };

        Ok((auth_url, auth_state))
    }

    pub async fn exchange_code_for_token(
        &self,
        code: String,
        state: AuthState,
    ) -> Result<TokenInfo> {
        let pkce_verifier = oauth2::PkceCodeVerifier::new(state.pkce_verifier);

        let token_result = self
            .client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(oauth2::reqwest::async_http_client)
            .await?;

        Ok(TokenInfo {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result
                .refresh_token()
                .map(|t| t.secret().clone()),
            expires_in: token_result
                .expires_in()
                .map(|d| d.as_secs()),
            token_type: "Bearer".to_string(),
        })
    }

    pub async fn refresh_token(&self, refresh_token: String) -> Result<TokenInfo> {
        let refresh_token = oauth2::RefreshToken::new(refresh_token);

        let token_result = self
            .client
            .exchange_refresh_token(&refresh_token)
            .request_async(oauth2::reqwest::async_http_client)
            .await?;

        Ok(TokenInfo {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result
                .refresh_token()
                .map(|t| t.secret().clone()),
            expires_in: token_result
                .expires_in()
                .map(|d| d.as_secs()),
            token_type: "Bearer".to_string(),
        })
    }

    pub async fn validate_token(&self, token: &str) -> Result<bool> {
        let response = self
            .http_client
            .get("https://graph.microsoft.com/v1.0/me")
            .bearer_auth(token)
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    pub async fn send_token_to_email_service(&self, token: &TokenInfo) -> Result<()> {
        let email_service_url = std::env::var("EMAIL_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8007".to_string());

        let mut payload = HashMap::new();
        payload.insert("access_token", &token.access_token);
        payload.insert("token_type", &token.token_type);

        let response = self
            .http_client
            .post(&format!("{}/api/v1/oauth/token", email_service_url))
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to send token to email service: {}",
                response.status()
            ));
        }

        Ok(())
    }
}
