use simple_server::{Method, Server, StatusCode};

fn main() {
    let host = "127.0.0.1";
    let port = "8080";

    println!("Starting server on localhost:8080");

    let server = Server::new(|request, mut response| {
        match (request.method(), request.uri().path()) {
            (&Method::GET, "/") => {
                Ok(response.body("
                    <a href=\"/login\">
                        <img src=\"https://steamcommunity-a.akamaihd.net/public/images/signinthroughsteam/sits_01.png\">
                    </a>
                ".as_bytes().to_vec())?)
            }
            (&Method::GET, "/login") => {
                let redirect_url = steam_auth::get_login_url("http://localhost:8080", "/callback").unwrap();

                // Redirect user to redirect_url
                response.status(StatusCode::FOUND);
                response.header("Location", redirect_url.as_str());
                Ok(response.body(Vec::new())?)
            }
            (&Method::GET, "/callback") => {
                // Parse query string data into auth_resp
                let form_string = request.uri().query().unwrap();
                dbg!(form_string);
                let auth_resp = serde_urlencoded::from_str(form_string).unwrap();

                // Check with the steam servers if the response was valid
                match steam_auth::verify_response(&reqwest::Client::new(), auth_resp) {
                    Ok(id) => Ok(response.body(format!("<h1>Success</h1><p>Steam ID: {}</p>", id).as_bytes().to_vec())?),
                    Err(e) => Ok(response.body(format!("<h1>Error</h1><p>Description: {}</p>", e).as_bytes().to_vec())?),
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
