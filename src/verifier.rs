use crate::{Error, STEAM_URL};

#[cfg(feature = "reqwest-09x")]
use futures::{Future, Stream, future::{self, Either}};

#[derive(Debug, Clone)]
pub struct Verifier {
    claimed_id: u64,
}

impl Verifier {
    pub fn from_querystring<S: AsRef<str>>(s: S) -> Result<(http::Request<Vec<u8>>, Self), Error> {
        let mut form: SteamAuthResponse = serde_urlencoded::from_str(s.as_ref()).unwrap(); // TODO: Unwrap
        form.mode = "check_authentication".to_owned();

        let verifier = {
            let url = url::Url::parse(&form.claimed_id).map_err(|_| Error::ParseSteamId)?;
            let mut segments = url.path_segments().ok_or(Error::ParseSteamId)?;
            let id_segment = segments.next_back().ok_or(Error::ParseSteamId)?;

            let claimed_id = id_segment.parse::<u64>().map_err(|_| Error::ParseSteamId)?;

            Self { claimed_id }
        };

        let form_data = dbg!(serde_urlencoded::to_string(form).unwrap()).into_bytes(); // TODO: Unwrap

        let req = http::Request::builder()
            .method(http::Method::POST)
            .uri(STEAM_URL)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_data)
            .unwrap(); // TODO: Unwrap

        Ok((req, verifier))
    }

    pub fn verify_response<S: Into<String>>(self, response_body: S) -> Result<u64, Error> {
        let valid = response_body
            .into()
            .split('\n')
            .map(|line| {
                let mut pair = line.split(':');
                (pair.next(), pair.next())
            })
            .filter_map(|kv| match kv {
                (Some(k), Some(v)) => Some((k, v)),
                _ => None,
            })
            .any(|(k, v)| k == "is_valid" && v == "true");

        if !valid {
            return Err(Error::AuthenticationFailed);
        }

        Ok(self.claimed_id)
    }

    #[cfg(feature = "reqwest-09x")]
    pub fn make_verify_request<S: AsRef<str>>(
        client: &reqwest::Client,
        querystring: S,
    ) -> Result<u64, Error> {
        let (req, verifier) = Self::from_querystring(querystring)?;

        client
            .post(&req.uri().to_string())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(req.body().clone())
            .send()
            .map_err(Error::Reqwest)
            .and_then(|mut response| {
                let text = response.text().map_err(Error::Reqwest)?;

                verifier.verify_response(text)
            })
    }

    #[cfg(feature = "reqwest-09x")]
    pub fn make_verify_request_async<S: AsRef<str>>(
        client: &reqwest::r#async::Client,
        querystring: S,
    ) -> impl Future<Item = u64, Error = Error> {
        let (req, verifier) = match Self::from_querystring(querystring) {
            Ok(rv) => rv,
            Err(e) => return Either::A(future::err(e)),
        };

        Either::B(
            client
                .post(&req.uri().to_string())
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(req.body().clone())
                .send()
                .map_err(Error::Reqwest)
                .and_then(|res| res.into_body().concat2().map_err(Error::Reqwest))
                .and_then(move |body| {
                    let s = std::str::from_utf8(&body)
                        .map_err(|_| Error::AuthenticationFailed)?
                        .to_owned();

                    verifier.verify_response(s)
                })
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
