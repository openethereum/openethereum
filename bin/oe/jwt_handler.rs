use chrono::Utc;
use ethereum_types::H256;
use hyper::{http::HeaderValue, Body, StatusCode};
use jsonwebtoken::{Algorithm, Validation};
use parity_rpc::{RequestMiddleware, RequestMiddlewareAction, Response};
use serde::Deserialize;

const IAT_WINDOW_SEC: i64 = 5;

#[derive(Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
struct Claims {
    iat: Option<i64>,
}

struct JwtHandler {
    secret: H256,
}

impl JwtHandler {
    pub fn new(secret: H256) -> Self {
        Self { secret }
    }
}

impl RequestMiddleware for JwtHandler {
    fn on_request(&self, request: hyper::Request<Body>) -> RequestMiddlewareAction {
        let as_string: fn(Option<&HeaderValue>) -> Option<String> =
            |header: Option<&HeaderValue>| {
                header.and_then(|val| val.to_str().ok().map(ToOwned::to_owned))
            };

        let forbidden: fn(&str) -> RequestMiddlewareAction = |content| {
            Response {
                code: StatusCode::FORBIDDEN,
                content_type: HeaderValue::from_static("text/plain; charset=utf-8"),
                content: content.to_string(),
            }
            .into()
        };

        // retrieve JWT token
        let token = as_string(request.headers().get("authorization"))
            .and_then(|val| val.strip_prefix("Bearer ").map(|val| val.to_owned()));
        if token.is_none() {
            return forbidden("missing token");
        }

        // parse the token
        let token = token.unwrap();
        let validation = {
            let mut validation = Validation::new(Algorithm::HS256);
            validation.validate_exp = false;
            validation
        };
        let claims =
            match jsonwebtoken::decode::<Claims>(&token, self.secret.as_bytes(), &validation) {
                Ok(data) => data.claims,
                Err(_) => return forbidden("invalid token"),
            };

        // verify `issued-at` claim
        if claims.iat.is_none() {
            return forbidden("missing issued-at");
        };
        let iat = claims.iat.unwrap();
        let now = Utc::now().timestamp();
        if now - iat > IAT_WINDOW_SEC {
            return forbidden("stale token");
        }
        if iat - now > IAT_WINDOW_SEC {
            return forbidden("future token");
        }

        // proceed to RPC handling
        RequestMiddlewareAction::Proceed {
            should_continue_on_invalid_cors: false,
            request,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{prelude::*, Stream};
    use jsonwebtoken::{encode, Header};
    use std::str;

    const SECRET: H256 = H256([
        0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
        0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 1u8,
    ]);
    const JWT_HANDLER: JwtHandler = JwtHandler { secret: SECRET };

    #[test]
    fn should_proceed_when_token_is_valid() {
        // given
        let iat = Utc::now().timestamp();
        let claims = Claims { iat: Some(iat) };
        let header = Header {
            alg: Algorithm::HS256,
            ..Default::default()
        };
        let jwt = encode(&header, &claims, SECRET.as_bytes()).expect("encoding failed.");
        let request = hyper::Request::get("example.com")
            .header("authorization", format!("Bearer {}", jwt))
            .body(Body::empty())
            .expect("request initialization failed");

        // when
        let action = JWT_HANDLER.on_request(request);

        // then
        match action {
            RequestMiddlewareAction::Proceed { .. } => {}
            RequestMiddlewareAction::Respond { .. } => {
                panic!("Middleware should proceed but have responded.")
            }
        }
    }

    #[test]
    fn should_respond_with_missing_token_when_token_is_missing() {
        // given
        let request = hyper::Request::get("example.com")
            .body(Body::empty())
            .expect("request initialization failed");

        // when
        let action = JWT_HANDLER.on_request(request);

        // then
        match action {
            RequestMiddlewareAction::Proceed { .. } => {
                panic!("Middleware should respond but have proceeded")
            }
            RequestMiddlewareAction::Respond {
                should_validate_hosts,
                response,
            } => {
                assert!(should_validate_hosts, "Invalid should_validate_hosts value");
                {
                    let response = response.wait().unwrap();

                    assert_eq!(
                        StatusCode::FORBIDDEN,
                        response.status(),
                        "Invalid status code"
                    );

                    let content = response.into_body().concat2().wait().unwrap().into_bytes();
                    let content = str::from_utf8(&content).unwrap();
                    assert_eq!("missing token", content, "Invalid content message")
                }
            }
        }
    }

    #[test]
    fn should_respond_with_invalid_token_when_token_is_invalid() {
        // given
        let jwt = "InvalidJWT".to_string();
        let request = hyper::Request::get("example.com")
            .header("authorization", format!("Bearer {}", jwt))
            .body(Body::empty())
            .expect("request initialization failed");

        // when
        let action = JWT_HANDLER.on_request(request);

        // then
        match action {
            RequestMiddlewareAction::Proceed { .. } => {
                panic!("Middleware should respond but have proceeded")
            }
            RequestMiddlewareAction::Respond {
                should_validate_hosts,
                response,
            } => {
                assert!(should_validate_hosts, "Invalid should_validate_hosts value");
                {
                    let response = response.wait().unwrap();

                    assert_eq!(
                        StatusCode::FORBIDDEN,
                        response.status(),
                        "Invalid status code"
                    );

                    let content = response.into_body().concat2().wait().unwrap().into_bytes();
                    let content = str::from_utf8(&content).unwrap();
                    assert_eq!("invalid token", content, "Invalid content message")
                }
            }
        }
    }

    #[test]
    fn should_respond_with_invalid_token_when_invalid_algorithm_used() {
        // given
        let iat = Utc::now().timestamp();
        let claims = Claims { iat: Some(iat) };
        let header = Header {
            alg: Algorithm::HS512,
            ..Default::default()
        };
        let jwt = encode(&header, &claims, SECRET.as_bytes()).expect("encoding failed.");
        let request = hyper::Request::get("example.com")
            .header("authorization", format!("Bearer {}", jwt))
            .body(Body::empty())
            .expect("request initialization failed");

        // when
        let action = JWT_HANDLER.on_request(request);

        // then
        match action {
            RequestMiddlewareAction::Proceed { .. } => {
                panic!("Middleware should respond but have proceeded")
            }
            RequestMiddlewareAction::Respond {
                should_validate_hosts,
                response,
            } => {
                assert!(should_validate_hosts, "Invalid should_validate_hosts value");
                {
                    let response = response.wait().unwrap();

                    assert_eq!(
                        StatusCode::FORBIDDEN,
                        response.status(),
                        "Invalid status code"
                    );

                    let content = response.into_body().concat2().wait().unwrap().into_bytes();
                    let content = str::from_utf8(&content).unwrap();
                    assert_eq!("invalid token", content, "Invalid content message")
                }
            }
        }
    }

    #[test]
    fn should_respond_with_missing_issued_at_when_iat_is_missing() {
        // given
        let claims = Claims { iat: None };
        let header = Header {
            alg: Algorithm::HS256,
            ..Default::default()
        };
        let jwt = encode(&header, &claims, SECRET.as_bytes()).expect("encoding failed.");
        let request = hyper::Request::get("example.com")
            .header("authorization", format!("Bearer {}", jwt))
            .body(Body::empty())
            .expect("request initialization failed");

        // when
        let action = JWT_HANDLER.on_request(request);

        // then
        match action {
            RequestMiddlewareAction::Proceed { .. } => {
                panic!("Middleware should respond but have proceeded")
            }
            RequestMiddlewareAction::Respond {
                should_validate_hosts,
                response,
            } => {
                assert!(should_validate_hosts, "Invalid should_validate_hosts value");
                {
                    let response = response.wait().unwrap();

                    assert_eq!(
                        StatusCode::FORBIDDEN,
                        response.status(),
                        "Invalid status code"
                    );

                    let content = response.into_body().concat2().wait().unwrap().into_bytes();
                    let content = str::from_utf8(&content).unwrap();
                    assert_eq!("missing issued-at", content, "Invalid content message")
                }
            }
        }
    }

    #[test]
    fn should_respond_with_stale_token_when_iat_is_too_old() {
        // given
        let iat = Utc::now().timestamp() - (IAT_WINDOW_SEC + 1);
        let claims = Claims { iat: Some(iat) };
        let header = Header {
            alg: Algorithm::HS256,
            ..Default::default()
        };
        let jwt = encode(&header, &claims, SECRET.as_bytes()).expect("encoding failed.");
        let request = hyper::Request::get("example.com")
            .header("authorization", format!("Bearer {}", jwt))
            .body(Body::empty())
            .expect("request initialization failed");

        // when
        let action = JWT_HANDLER.on_request(request);

        // then
        match action {
            RequestMiddlewareAction::Proceed { .. } => {
                panic!("Middleware should respond but have proceeded")
            }
            RequestMiddlewareAction::Respond {
                should_validate_hosts,
                response,
            } => {
                assert!(should_validate_hosts, "Invalid should_validate_hosts value");
                {
                    let response = response.wait().unwrap();

                    assert_eq!(
                        StatusCode::FORBIDDEN,
                        response.status(),
                        "Invalid status code"
                    );

                    let content = response.into_body().concat2().wait().unwrap().into_bytes();
                    let content = str::from_utf8(&content).unwrap();
                    assert_eq!("stale token", content, "Invalid content message")
                }
            }
        }
    }

    #[test]
    fn should_respond_with_future_token_when_iat_is_in_future() {
        // given
        let iat = Utc::now().timestamp() + (IAT_WINDOW_SEC + 2);
        let claims = Claims { iat: Some(iat) };
        let header = Header {
            alg: Algorithm::HS256,
            ..Default::default()
        };
        let jwt = encode(&header, &claims, SECRET.as_bytes()).expect("encoding failed.");
        let request = hyper::Request::get("example.com")
            .header("authorization", format!("Bearer {}", jwt))
            .body(Body::empty())
            .expect("request initialization failed");

        // when
        let action = JWT_HANDLER.on_request(request);

        // then
        match action {
            RequestMiddlewareAction::Proceed { .. } => {
                panic!("Middleware should respond but have proceeded")
            }
            RequestMiddlewareAction::Respond {
                should_validate_hosts,
                response,
            } => {
                assert!(should_validate_hosts, "Invalid should_validate_hosts value");
                {
                    let response = response.wait().unwrap();

                    assert_eq!(
                        StatusCode::FORBIDDEN,
                        response.status(),
                        "Invalid status code"
                    );

                    let content = response.into_body().concat2().wait().unwrap().into_bytes();
                    let content = str::from_utf8(&content).unwrap();
                    assert_eq!("future token", content, "Invalid content message")
                }
            }
        }
    }
}
