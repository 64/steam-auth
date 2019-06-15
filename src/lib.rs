//! Allows you to implement a 'login with steam' feature on your website.
//!
//! ## Usage
//!
//! First, obtain the URL to which users should be redirected to start the login process:
//!
//! ```rust
//! # fn main() {
//! let redirect_url = steam_auth::get_login_url("http://localhost:8080", "/callback").unwrap();
//! # }
//! ```
//!
//! After redirecting the user to this URL, they will be returned to `/callback` with some data in the query string that needs to be deserialized into a `SteamAuthResponse`. Then, verify the data (this makes an HTTP request to the steam servers):
//!
//! ```rust
//! # fn main() {
//! # let auth_response = serde_urlencoded::from_str("openid.ns=http%3A%2F%2Fspecs.openid.net%2Fauth%2F2.0&openid.mode=id_res&openid.op_endpoint=https%3A%2F%2Fsteamcommunity.com%2Fopenid%2Flogin&openid.claimed_id=https%3A%2F%2Fsteamcommunity.com%2Fopenid%2Fid%2F92345666790633291&openid.identity=https%3A%2F%2Fsteamcommunity.com%2Fopenid%2Fid%2F12333456789000000&openid.return_to=http%3A%2F%2Flocalhost%3A8080%2Fcallback&openid.response_nonce=2019-06-15T00%3A36%3A00Z7nVIS5lDAcZe%2FT0gT4%2BQNQyexyA%3D&openid.assoc_handle=1234567890&openid.signed=signed%2Cop_endpoint%2Cclaimed_id%2Cidentity%2Creturn_to%2Cresponse_nonce%2Cassoc_handle&openid.sig=BK0zC%2F%2FKzERs7N%2BNlDO0aL06%2BBA%3D").unwrap();
//! /* deserialize query string into auth_response: SteamAuthResponse */
//! match steam_auth::verify_response(&reqwest::Client::new(), auth_response) {
//!     Ok(id) => println!("Successfully logged in user with STEAMID64: {}", id),
//!     Err(e) => println!("Login unsuccessful: {}", e),
//! }
//! # }
//! ```
//!
//! There's also an asynchronous variant on `steam_auth::verify_response_async`.
//!
//! See the example server for more details.

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;

use futures::{Future, Stream};
use url::Url;

const STEAM_URL: &str = "https://steamcommunity.com/openid/login";

#[derive(Serialize)]
struct SteamAuthRequest {
    #[serde(rename = "openid.ns")]
    ns: &'static str,
    #[serde(rename = "openid.identity")]
    identity: &'static str,
    #[serde(rename = "openid.claimed_id")]
    claimed_id: &'static str,
    #[serde(rename = "openid.mode")]
    mode: &'static str,
    #[serde(rename = "openid.return_to")]
    return_to: String,
    #[serde(rename = "openid.realm")]
    realm: String,
}

impl SteamAuthRequest {
    pub fn new(site_url: String, return_to_joined: String) -> Self {
        Self {
            ns: "http://specs.openid.net/auth/2.0",
            identity: "http://specs.openid.net/auth/2.0/identifier_select",
            claimed_id: "http://specs.openid.net/auth/2.0/identifier_select",
            mode: "checkid_setup",
            realm: site_url,
            return_to: return_to_joined,
        }
    }
}

/// Represents the data that is returned by Steam to the callback URL.
#[derive(Deserialize, Serialize, Debug)]
pub struct SteamAuthResponse {
    #[serde(rename = "openid.ns")]
    ns: String,
    #[serde(rename = "openid.mode")]
    mode: String,
    #[serde(rename = "openid.op_endpoint")]
    op_endpoint: String,
    #[serde(rename = "openid.claimed_id")]
    claimed_id: String,
    #[serde(rename = "openid.identity")]
    identity: Option<String>,
    #[serde(rename = "openid.return_to")]
    return_to: String,
    #[serde(rename = "openid.response_nonce")]
    response_nonce: String,
    #[serde(rename = "openid.invalidate_handle")]
    invalidate_handle: Option<String>,
    #[serde(rename = "openid.assoc_handle")]
    assoc_handle: String,
    #[serde(rename = "openid.signed")]
    signed: String,
    #[serde(rename = "openid.sig")]
    sig: String,
}

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "reqwest error: {}", _0)]
    /// There was an error during the verify request
    Reqwest(reqwest::Error),
    #[fail(display = "bad site or return url: {}", _0)]
    /// The site or return URL was incorrect
    BadUrl(url::ParseError),
    #[fail(display = "failed to parse SteamAuthRequest (please file bug): {}", _0)]
    /// Internal error serializing the query string - should never happen.
    ParseQueryString(serde_urlencoded::ser::Error),
    #[fail(display = "authentication failed")]
    /// The authentication failed because the data provided to the callback was invalid
    AuthenticationFailed,
    #[fail(display = "failed to parse steam id")]
    /// There was an error parsing the Steam ID returned to the callback
    ParseSteamId,
}

fn parse_verify_response(claimed_id: &str, response: String) -> Result<u64, Error> {
    // Parse ID and return it
    let valid = response
        .split('\n')
        .map(|line| {
            let mut pair = line.split(':');
            (pair.next(), pair.next())
        })
        .filter_map(|(k, v)| k.and_then(|k| v.map(|v| (k, v))))
        .any(|(k, v)| k == "is_valid" && v == "true");

    if !valid {
        return Err(Error::AuthenticationFailed);
    }

    // Extract Steam ID
    let url = Url::parse(&claimed_id).map_err(|_| Error::ParseSteamId)?;
    let mut segments = url.path_segments().ok_or(Error::ParseSteamId)?;
    let id_segment = segments.next_back().ok_or(Error::ParseSteamId)?;

    id_segment.parse::<u64>().map_err(|_| Error::ParseSteamId)
}

/// Obtains the URL to which users should be redirected to start the login process
pub fn get_login_url<T: AsRef<str>, U: AsRef<str>>(
    site_url: T,
    return_url: U,
) -> Result<url::Url, Error> {
    let joined = Url::parse(site_url.as_ref())
        .map_err(Error::BadUrl)?
        .join(return_url.as_ref())
        .map_err(Error::BadUrl)?;

    let openid = SteamAuthRequest::new(site_url.as_ref().to_owned(), joined.into_string());

    let qs = serde_urlencoded::to_string(&openid).map_err(Error::ParseQueryString)?;

    // TODO: Remove unwrap
    let mut url = Url::parse(STEAM_URL).map_err(Error::BadUrl)?; // Shouldn't happen

    url.set_query(Some(&qs));

    Ok(url)
}

/// Verifies callback data (asynchronous)
pub fn verify_response_async(
    client: &reqwest::r#async::Client,
    mut form: SteamAuthResponse,
) -> impl Future<Item = u64, Error = Error> {
    form.mode = "check_authentication".to_owned();

    client
        .post(STEAM_URL)
        .form(&form)
        .send()
        .map_err(Error::Reqwest)
        .and_then(|res| res.into_body().concat2().map_err(Error::Reqwest))
        .and_then(move |body| {
            let s = std::str::from_utf8(&body)
                .map_err(|_| Error::AuthenticationFailed)?
                .to_owned();

            parse_verify_response(&form.claimed_id, s)
        })
}

/// Verifies callback data (synchronous)
pub fn verify_response(
    client: &reqwest::Client,
    mut form: SteamAuthResponse,
) -> Result<u64, Error> {
    form.mode = "check_authentication".to_owned();

    client
        .post(STEAM_URL)
        .form(&form)
        .send()
        .map_err(Error::Reqwest)
        .and_then(|mut response| {
            let text = response.text().map_err(Error::Reqwest)?;

            parse_verify_response(&form.claimed_id, text)
        })
}
