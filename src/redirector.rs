use crate::{Error, STEAM_URL};
use url::Url;

#[derive(Debug, Clone)]
pub struct Redirector {
    url: Url,
}

impl Redirector {
    pub fn new<T: AsRef<str>, U: AsRef<str>>(site_url: T, return_url: U) -> Result<Self, Error> {
        let joined = Url::parse(site_url.as_ref())
            .map_err(Error::BadUrl)?
            .join(return_url.as_ref())
            .map_err(Error::BadUrl)?;

        let openid = SteamAuthRequest::new(site_url.as_ref(), joined.as_str());

        let qs = serde_urlencoded::to_string(&openid).map_err(Error::ParseQueryString)?;

        // TODO: Remove unwrap
        // Shouldn't happen
        let mut url = Url::parse(STEAM_URL).map_err(Error::BadUrl)?;

        url.set_query(Some(&qs));

        Ok(Self { url })
    }

    pub fn create_response(&self) -> http::Result<http::Response<()>> {
        http::Response::builder()
            .status(http::StatusCode::FOUND)
            .header("Location", self.url.as_str())
            .body(())
    }

    pub fn url(&self) -> &Url {
        &self.url
    }
}

#[derive(Serialize)]
struct SteamAuthRequest<'a> {
    #[serde(rename = "openid.ns")]
    ns: &'static str,
    #[serde(rename = "openid.identity")]
    identity: &'static str,
    #[serde(rename = "openid.claimed_id")]
    claimed_id: &'static str,
    #[serde(rename = "openid.mode")]
    mode: &'static str,
    #[serde(rename = "openid.return_to")]
    return_to: &'a str,
    #[serde(rename = "openid.realm")]
    realm: &'a str,
}

impl<'a> SteamAuthRequest<'a> {
    fn new(site_url: &'a str, return_to_joined: &'a str) -> Self {
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
