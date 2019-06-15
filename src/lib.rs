//! Allows you to implement a 'login with steam' feature on your website.
//!
//! ## Usage
//!
//! TODO
//! ```rust
//! # fn main() {
//! # let auth_response = serde_urlencoded::from_str("openid.ns=http%3A%2F%2Fspecs.openid.net%2Fauth%2F2.0&openid.mode=id_res&openid.op_endpoint=https%3A%2F%2Fsteamcommunity.com%2Fopenid%2Flogin&openid.claimed_id=https%3A%2F%2Fsteamcommunity.com%2Fopenid%2Fid%2F92345666790633291&openid.identity=https%3A%2F%2Fsteamcommunity.com%2Fopenid%2Fid%2F12333456789000000&openid.return_to=http%3A%2F%2Flocalhost%3A8080%2Fcallback&openid.response_nonce=2019-06-15T00%3A36%3A00Z7nVIS5lDAcZe%2FT0gT4%2BQNQyexyA%3D&openid.assoc_handle=1234567890&openid.signed=signed%2Cop_endpoint%2Cclaimed_id%2Cidentity%2Creturn_to%2Cresponse_nonce%2Cassoc_handle&openid.sig=BK0zC%2F%2FKzERs7N%2BNlDO0aL06%2BBA%3D").unwrap();
//! // deserialize query string into auth_response: SteamAuthResponse
//! # }
//! ```
//!
//! See the [example server](https://github.com/64/steam-auth/blob/master/examples/server.rs) for more details.

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;

mod redirector;
mod verifier;

pub use verifier::Verifier;
pub use redirector::Redirector;

pub(crate) const STEAM_URL: &str = "https://steamcommunity.com/openid/login";

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "reqwest error: {}", _0)]
    #[cfg(feature = "reqwest-09x")]
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

#[cfg(feature = "reqwest-0_9")]
pub fn verify_response_async(
    client: &reqwest::r#async::Client,
    mut form: SteamAuthResponse,
) -> impl futures::Future<Item = u64, Error = Error> {
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
