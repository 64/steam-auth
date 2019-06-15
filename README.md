[![Docs.rs](https://docs.rs/steam-auth/badge.svg)](https://docs.rs/steam-auth)
[![Build Status](https://travis-ci.org/64/steam-auth.svg?branch=master)](https://travis-ci.org/64/steam-auth)

# steam-auth

Allows you to implement a 'login with steam' feature on your website.

WIP REWRITE BRANCH

### Usage

First, obtain the URL to which users should be redirected to start the login process:

```rust
let redirect_url = steam_auth::get_login_url("http://localhost:8080", "/callback").unwrap();
```

After redirecting the user to this URL, they will be returned to `/callback` with some data in the query string that needs to be deserialized into a `SteamAuthResponse`. Then, verify the data (this makes an HTTP request to the steam servers):

```rust
// deserialize query string into auth_response: SteamAuthResponse
match steam_auth::verify_response(&reqwest::Client::new(), auth_response) {
    Ok(id) => println!("Successfully logged in user with STEAMID64: {}", id),
    Err(e) => println!("Login unsuccessful: {}", e),
}
```

There's also an asynchronous variant on `steam_auth::verify_response_async`.

See the [example server](https://github.com/64/steam-auth/blob/master/examples/server.rs) for more details.

MIT Licensed. Pull requests and contributions welcome.
