[![Docs.rs](https://docs.rs/steam-auth/badge.svg)](https://docs.rs/steam-auth)
[![Build Status](https://travis-ci.org/64/steam-auth.svg?branch=master)](https://travis-ci.org/64/steam-auth)

# steam-auth

Allows you to implement a 'login with steam' feature on your website.

### Usage

The easiest way to use this crate is with the `reqwest-09x` feature which allows the library to
make HTTP requests on your behalf. Otherwise, you will need to do that manually.

Using the `reqwest-09x` feature:
```rust
// First, create a redirector
let redirector = Redirector::new("http://localhost:8080", "/callback").unwrap();

// When a user wants to log in with steam, (e.g when they land on the `/login` route),
// redirect them to this URL:
let redirect_url = redirector.url();

// Once they've finished authenticating, they will be returned to `/callback` with some data in
// the query string that needs to be parsed and then verified by sending an HTTP request to the steam
// servers.
match Verifier::make_verify_request(&reqwest::Client::new(), querystring) {
    Ok(steam_id) => println!("Successfully logged in user with steam ID 64 {}", steam_id),
    Err(e) => eprintln!("There was an error authenticating: {}", e),
}
```

There is also an asynchronous variant: `Verifier::make_verify_request_async` which returns a
future.

If you don't want to depend on request, you'll need to send the HTTP request yourself. See the
[example server](https://github.com/64/steam-auth/blob/master/examples/server.rs) and the
`Verifier` documentation for more details on how this can be done.

MIT Licensed. Pull requests and contributions welcome.
