use simple_server::{Method, Server, StatusCode};

fn main() {
    let host = "127.0.0.1";
    let port = "8080";

    println!("Starting server on localhost:8080");

    let redirector = steam_auth::Redirector::new("http://localhost:8080", "/callback").unwrap();

    let server = Server::new(move |request, mut response| {
        match (request.method(), request.uri().path()) {
            (&Method::GET, "/") => {
                Ok(response.body("
                    <a href=\"/login\">
                        <img src=\"https://steamcommunity-a.akamaihd.net/public/images/signinthroughsteam/sits_01.png\">
                    </a>
                ".as_bytes().to_vec())?)
            }
            (&Method::GET, "/login") => {
                // Redirect user to redirect_url
                response.status(StatusCode::FOUND);
                response.header("Location", redirector.url().as_str());
                Ok(response.body(Vec::new())?)
            }
            (&Method::GET, "/callback") => {
                // Parse query string data into auth_resp
                let qs = request.uri().query().unwrap();

                // Check with the steam servers if the response was valid
                match steam_auth::Verifier::make_verify_request(&reqwest::Client::new(), qs) {
                    Ok(id) => Ok(response.body(format!("<h1>Success</h1><p>Steam ID: {}</p>", id).as_bytes().to_vec())?),
                    Err(e) => Ok(response.body(format!("<h1>Error</h1><p>Description: {}</p>", dbg!(e)).as_bytes().to_vec())?),
                }
            }
            (_, _) => {
                response.status(StatusCode::NOT_FOUND);
                Ok(response.body("<h1>404</h1><p>Not found!</p>".as_bytes().to_vec())?)
            }
        }
    });

    server.listen(host, port);
}
