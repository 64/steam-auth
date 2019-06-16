use crate::{Error, STEAM_URL};

#[cfg(feature = "reqwest-09x")]
use futures::{
    future::{self, Either},
    Future, Stream,
};

#[derive(Debug, Clone)]
/// Verifies the login details returned after users have gone through the 'sign in with Steam' page
/// # Example
/// ```
/// # use steam_auth::Verifier;
/// # struct Response; impl Response { fn new() -> Self { Self } fn body(&self) -> &'static
/// # str { "foo" } }
/// # fn main() {
/// # let qs = "openid.ns=http%3A%2F%2Fspecs.openid.net%2Fauth%2F2.0&openid.mode=id_res&openid.op_endpoint=https%3A%2F%2Fsteamcommunity.com%2Fopenid%2Flogin&openid.claimed_id=https%3A%2F%2Fsteamcommunity.com%2Fopenid%2Fid%2F92345666790633291&openid.identity=https%3A%2F%2Fsteamcommunity.com%2Fopenid%2Fid%2F12333456789000000&openid.return_to=http%3A%2F%2Flocalhost%3A8080%2Fcallback&openid.response_nonce=2019-06-15T00%3A36%3A00Z7nVIS5lDAcZe%2FT0gT4%2BQNQyexyA%3D&openid.assoc_handle=1234567890&openid.signed=signed%2Cop_endpoint%2Cclaimed_id%2Cidentity%2Creturn_to%2Cresponse_nonce%2Cassoc_handle&openid.sig=BK0zC%2F%2FKzERs7N%2BNlDO0aL06%2BBA%3D";
/// let (req, verifier) = Verifier::from_querystring(qs).unwrap();
/// // send off req, get back response
/// # let response = Response;
/// match verifier.verify_response(response.body()) {
///     Ok(steam_id) => (), // got steam id
///     Err(e) => (), // Auth failure
/// }
/// # }
/// ```
pub struct Verifier {
    claimed_id: u64,
}

impl Verifier {
    /// Constructs a Verifier and a HTTP request from a query string. You must use the method,
    /// headers, URI and body from the returned `http::Request` struct.
    pub fn from_querystring<S: AsRef<str>>(s: S) -> Result<(http::Request<Vec<u8>>, Self), Error> {
        let mut form: SteamAuthResponse =
            serde_urlencoded::from_str(s.as_ref()).map_err(Error::Deserialize)?;

        form.mode = "check_authentication".to_owned();

        let verifier = {
            let url = url::Url::parse(&form.claimed_id).map_err(|_| Error::ParseSteamId)?;
            let mut segments = url.path_segments().ok_or(Error::ParseSteamId)?;
            let id_segment = segments.next_back().ok_or(Error::ParseSteamId)?;

            let claimed_id = id_segment.parse::<u64>().map_err(|_| Error::ParseSteamId)?;

            Self { claimed_id }
        };

        let form_data = serde_urlencoded::to_string(form)
            .map_err(Error::Serialize)?
            .into_bytes();

        let req = http::Request::builder()
            .method(http::Method::POST)
            .uri(STEAM_URL)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_data)
            .map_err(Error::BuildHttpStruct)?;

        Ok((req, verifier))
    }

    /// Verifies the response from the steam servers.
    pub fn verify_response<S: Into<String>>(self, response_body: S) -> Result<u64, Error> {
        let is_valid = response_body
            .into()
            .split('\n')
            .filter_map(|line| {
                // Allow values to contain colons, but not keys
                let mut pair = line.splitn(2, ':');
                Some((pair.next()?, pair.next()?))
            })
            .any(|(k, v)| k == "is_valid" && v == "true");

        match is_valid {
            true => Ok(self.claimed_id),
            false => Err(Error::AuthenticationFailed),
        }
    }

    #[cfg(feature = "reqwest-09x")]
    /// Constructs and sends a synchronous verification request. Requires the `reqwest-09x`
    /// feature.
    pub fn make_verify_request<S: AsRef<str>>(
        client: &reqwest::Client,
        querystring: S,
    ) -> Result<u64, Error> {
        let (req, verifier) = Self::from_querystring(querystring)?;

        let (parts, body) = req.into_parts();

        client
            .post(&parts.uri.to_string())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .map_err(Error::Reqwest)
            .and_then(|mut response| {
                let text = response.text().map_err(Error::Reqwest)?;

                verifier.verify_response(text)
            })
    }

    #[cfg(feature = "reqwest-09x")]
    /// Constructs and sends an asynchronous verification request. Requires the `reqwest-09x`
    /// feature.
    pub fn make_verify_request_async<S: AsRef<str>>(
        client: &reqwest::r#async::Client,
        querystring: S,
    ) -> impl Future<Item = u64, Error = Error> {
        let (req, verifier) = match Self::from_querystring(querystring) {
            Ok(rv) => rv,
            Err(e) => return Either::A(future::err(e)),
        };

        let (parts, body) = req.into_parts();

        Either::B(
            client
                .post(&parts.uri.to_string())
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body)
                .send()
                .map_err(Error::Reqwest)
                .and_then(|res| res.into_body().concat2().map_err(Error::Reqwest))
                .and_then(move |body| {
                    let s = std::str::from_utf8(&body)
                        .map_err(|_| Error::AuthenticationFailed)?
                        .to_owned();

                    verifier.verify_response(s)
                }),
        )
    }
}

#[derive(Clone, Deserialize, Serialize, Debug)]
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
