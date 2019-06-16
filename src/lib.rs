//! Allows you to implement a 'login with steam' feature on your website.
//!
//! ## Usage
//!
//! The easiest way to use this crate is with the `reqwest-09x` feature which allows the library to
//! make HTTP requests on your behalf. Otherwise, you will need to do that manually.
//!
//! Using the `reqwest-09x` feature:
//! ```rust
//! # use steam_auth::{Redirector, Verifier};
//! # fn main() {
//! // First, create a redirector
//! let redirector = Redirector::new("http://localhost:8080", "/callback").unwrap();
//!
//! // When a user wants to log in with steam, (e.g when they land on the `/login` route),
//! // redirect them to this URL:
//! let redirect_url = redirector.url();
//!
//! // Once they've finished authenticating, they will be returned to `/callback` with some data in
//! // the query string that needs to be parsed and then verified by sending an HTTP request to the steam
//! // servers.
//! # let querystring = "openid.ns=http%3A%2F%2Fspecs.openid.net%2Fauth%2F2.0&openid.mode=id_res&openid.op_endpoint=https%3A%2F%2Fsteamcommunity.com%2Fopenid%2Flogin&openid.claimed_id=https%3A%2F%2Fsteamcommunity.com%2Fopenid%2Fid%2F92345666790633291&openid.identity=https%3A%2F%2Fsteamcommunity.com%2Fopenid%2Fid%2F12333456789000000&openid.return_to=http%3A%2F%2Flocalhost%3A8080%2Fcallback&openid.response_nonce=2019-06-15T00%3A36%3A00Z7nVIS5lDAcZe%2FT0gT4%2BQNQyexyA%3D&openid.assoc_handle=1234567890&openid.signed=signed%2Cop_endpoint%2Cclaimed_id%2Cidentity%2Creturn_to%2Cresponse_nonce%2Cassoc_handle&openid.sig=BK0zC%2F%2FKzERs7N%2BNlDO0aL06%2BBA%3D";
//! match Verifier::make_verify_request(&reqwest::Client::new(), querystring) {
//!     Ok(steam_id) => println!("Successfully logged in user with steam ID 64 {}", steam_id),
//!     Err(e) => eprintln!("There was an error authenticating: {}", e),
//! }
//! # }
//! ```
//!
//! There is also an asynchronous variant: `Verifier::make_verify_request_async` which returns a
//! future.
//!
//! If you don't want to depend on request, you'll need to send the HTTP request yourself. See the
//! [example server](https://github.com/64/steam-auth/blob/master/examples/server.rs) and the
//! `Verifier` documentation for more details on how this can be done.

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;

mod redirector;
mod verifier;

pub use redirector::Redirector;
pub use verifier::Verifier;

pub(crate) const STEAM_URL: &str = "https://steamcommunity.com/openid/login";

#[derive(Debug, Fail)]
pub enum Error {
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
    #[fail(display = "failed to build HTTP request or response: {}", _0)]
    BuildHttpStruct(http::Error),
    #[fail(display = "error serializing url encoded data: {}", _0)]
    Serialize(serde_urlencoded::ser::Error),
    #[fail(display = "error deserializing url encoded data: {}", _0)]
    Deserialize(serde_urlencoded::de::Error),
    #[fail(display = "reqwest error: {}", _0)]
    #[cfg(feature = "reqwest-09x")]
    /// There was an error during the verify request
    Reqwest(reqwest::Error),
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
