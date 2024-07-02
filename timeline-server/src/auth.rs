use anyhow::anyhow;
use openidconnect::core::{
    CoreAuthenticationFlow, CoreClient, CoreGenderClaim, CoreProviderMetadata,
};
use openidconnect::{
    AccessTokenHash, AdditionalClaims, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    EmptyAdditionalClaims, IdTokenClaims, IssuerUrl, Nonce, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, Scope, UserInfoClaims,
};
use serde::{Deserialize, Serialize};

use openidconnect::reqwest::async_http_client;
use url::Url;

use std::error::Error;

#[derive(Clone, Debug, Deserialize)]
pub struct AuthConfig {
    issuer_url: Url,
    redirect_url: RedirectUrl,
    client_id: String,
    client_secret: ClientSecret,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthContent {
    pub redirect_url: Url,
    pub verify: AuthVerify,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthVerify {
    pkce_verifier: PkceCodeVerifier,
    nonce: Nonce,
    pub csrf_token: CsrfToken,
}

#[derive(Debug, Deserialize, Serialize)]
struct GroupClaims {
    scopes: Vec<String>,
    groups: Vec<String>,
}
impl AdditionalClaims for GroupClaims {}

// Use OpenID Connect Discovery to fetch the provider metadata.
use openidconnect::{OAuth2TokenResponse, TokenResponse};

#[tracing::instrument]
pub async fn construct_client(auth_config: AuthConfig) -> Result<CoreClient, Box<dyn Error>> {
    let provider_metadata = CoreProviderMetadata::discover_async(
        //&IssuerUrl::new("https://accounts.example.com".to_string())?,
        IssuerUrl::from_url(auth_config.issuer_url),
        async_http_client,
    )
    .await?;

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        //ClientId::new("client_id".to_string()),
        //Some(ClientSecret::new("client_secret".to_string())),
        ClientId::new(auth_config.client_id),
        Some(auth_config.client_secret),
    )
    // Set the URL the user will be redirected to after the authorization process.
    //.set_redirect_uri(RedirectUrl::new("http://redirect".to_string())?);
    .set_redirect_uri(auth_config.redirect_url);
    return Ok(client);
}

#[tracing::instrument]
pub async fn get_auth_url(client: CoreClient) -> AuthContent {
    // Create an OpenID Connect client by specifying the client ID, client secret, authorization URL
    // and token URL.

    // Generate a PKCE challenge.
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate the full authorization URL.
    let (auth_url, csrf_token, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        // Set the desired scopes.
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("email".to_string()))
        // Set the PKCE code challenge.
        .set_pkce_challenge(pkce_challenge)
        .url();

    // This is the URL you should redirect the user to, in order to trigger the authorization
    // process.
    let ac = AuthContent {
        redirect_url: auth_url,
        verify: AuthVerify {
            csrf_token,
            pkce_verifier,
            nonce,
        },
    };
    return ac;
}

#[tracing::instrument]
pub async fn next(
    client: CoreClient,
    auth_verify: AuthVerify,
    auth_code: String,
) -> anyhow::Result<IdTokenClaims<EmptyAdditionalClaims, CoreGenderClaim>> {
    // Once the user has been redirected to the redirect URL, you'll have access to the
    // authorization code. For security reasons, your code should verify that the `state`
    // parameter returned by the server matches `csrf_state`.

    // Now you can exchange it for an access token and ID token.
    let token_response = client
        .exchange_code(AuthorizationCode::new(auth_code))
        // Set the PKCE code verifier.
        .set_pkce_verifier(auth_verify.pkce_verifier)
        .request_async(async_http_client)
        .await?;

    // Extract the ID token claims after verifying its authenticity and nonce.
    let id_token = token_response
        .id_token()
        .ok_or_else(|| anyhow!("Server did not return an ID token"))?;
    let claims = id_token.claims(&client.id_token_verifier(), &auth_verify.nonce)?;

    // Verify the access token hash to ensure that the access token hasn't been substituted for
    // another user's.
    if let Some(expected_access_token_hash) = claims.access_token_hash() {
        let actual_access_token_hash =
            AccessTokenHash::from_token(token_response.access_token(), &id_token.signing_alg()?)?;
        if actual_access_token_hash != *expected_access_token_hash {
            return Err(anyhow!("Invalid access token"));
        }
    }

    // The authenticated user's identity is now available. See the IdTokenClaims struct for a
    // complete listing of the available claims.
    println!(
        "User {} with e-mail address {} has authenticated successfully",
        claims.subject().as_str(),
        claims
            .email()
            .map(|email| email.as_str())
            .unwrap_or("<not provided>"),
    );
    // The user_info request uses the AccessToken returned in the token response. To parse custom
    // claims, use UserInfoClaims directly (with the desired type parameters) rather than using the
    // CoreUserInfoClaims type alias.
    let userinfo_claims: UserInfoClaims<GroupClaims, CoreGenderClaim> = client
        .user_info(token_response.access_token().to_owned(), None)
        .map_err(|err| anyhow!("No user info endpoint: {:?}", err))?
        .request_async(async_http_client)
        .await
        .map_err(|err| anyhow!("Failed requesting user info: {:?}", err))?;

    println!("Userinfo: {:?},", userinfo_claims);

    // If available, we can use the UserInfo endpoint to request additional information.

    // The user_info request uses the AccessToken returned in the token response. To parse custom
    // claims, use UserInfoClaims directly (with the desired type parameters) rather than using the
    // CoreUserInfoClaims type alias.

    // See the OAuth2TokenResponse trait for a listing of other available fields such as
    // access_token() and refresh_token().
    return Ok(claims.clone());
}
