//! Executable contracts for the maintained, synthetic output gallery.
use serde_json::Value;

pub(crate) fn example(country: &str, index: usize) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("docs/examples/{country}.md"));
    let doc = std::fs::read_to_string(path).unwrap();
    let block = doc
        .split("```json\n")
        .nth(index + 1)
        .unwrap()
        .split("\n```")
        .next()
        .unwrap();
    serde_json::from_str(block).unwrap()
}

#[tokio::test]
async fn documented_registry_failures_match_public_clients() {
    use crate::*;
    use tokio::io::AsyncWriteExt;
    for country in [
        "at", "be", "cz", "ee", "fi", "fr", "gr", "ie", "is", "lt", "lv", "no", "pl", "ro", "sk",
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let (status, content_type, body) = match country {
            "cz" => (
                404,
                "application/json",
                r#"{"kod":"NENALEZENO","popis":"Requested economic subject not found"}"#,
            ),
            "ro" => (
                200,
                "application/json",
                r#"{"cod":200,"message":"SUCCESS","found":[],"notFound":[1234]}"#,
            ),
            _ => (404, "text/plain", ""),
        };
        let server = super::http::spawn_server(async move {
            for _ in 0..if country == "pl" { 2 } else { 1 } {
                let (mut socket, _) = listener.accept().await.unwrap();
                super::http::read_request(&mut socket).await.unwrap();
                socket.write_all(format!("HTTP/1.1 {status} Test\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            }
        });
        let positive = example(country, 0);
        let id = positive["taxId"].as_str().unwrap();
        macro_rules! lookup {
            ($client:ident) => {
                $client::for_test(endpoint, Default::default())
                    .lookup(id)
                    .await
                    .map(|_| ())
                    .unwrap_err()
            };
        }
        let error = match country {
            "at" => lookup!(AtRegistryClient),
            "be" => lookup!(BeRegistryClient),
            "cz" => lookup!(CzRegistryClient),
            "ee" => lookup!(EeRegistryClient),
            "fi" => lookup!(FiRegistryClient),
            "fr" => lookup!(FrRegistryClient),
            "gr" => lookup!(GrRegistryClient),
            "ie" => lookup!(IeRegistryClient),
            "is" => lookup!(IsRegistryClient),
            "lt" => lookup!(LtRegistryClient),
            "lv" => lookup!(LvRegistryClient),
            "no" => lookup!(NoRegistryClient),
            "pl" => lookup!(PlRegistryClient),
            "sk" => lookup!(SkRegistryClient),
            "ro" => RoRegistryClient::for_test(endpoint, Default::default())
                .lookup_at(id, jiff::civil::Date::new(2025, 2, 14).unwrap())
                .await
                .unwrap_err(),
            _ => unreachable!(),
        };
        assert_eq!(
            serde_json::to_value(error).unwrap(),
            example(country, 1),
            "{country} documented error drifted"
        );
        server.await.unwrap();
    }
}
