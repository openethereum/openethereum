use chrono::Utc;
use jsonrpc_http_server::{
    hyper::{self, http::HeaderValue, Body, StatusCode},
    RequestMiddleware, RequestMiddlewareAction, Response,
};
use jsonwebtoken::{Algorithm, Validation};
use std::marker::{Send, Sync};

use crate::{clock::Clock, Secret};

const IAT_WINDOW_SEC: i64 = 5;

#[derive(serde::Deserialize, Default)]
#[cfg_attr(test, derive(serde::Serialize))]
struct Claims {
    iat: Option<i64>,
    exp: Option<i64>,
}

pub struct JwtHandler<C>
where
    C: Clock + Sync + Send + 'static,
{
    clock: C,
    secret: Secret,
}

impl<C> JwtHandler<C>
where
    C: Clock + Sync + Send + 'static,
{
    pub fn with_clock(clock: C, secret: Secret) -> Self {
        Self { clock, secret }
    }
}

impl JwtHandler<Utc> {
    pub fn new(secret: Secret) -> Self {
        JwtHandler::with_clock(Utc, secret)
    }
}

impl<C> RequestMiddleware for JwtHandler<C>
where
    C: Clock + Sync + Send + 'static,
{
    fn on_request(&self, request: hyper::Request<Body>) -> RequestMiddlewareAction {
        let as_string: fn(Option<&HeaderValue>) -> Option<String> =
            |header: Option<&HeaderValue>| {
                header.and_then(|val| val.to_str().ok().map(ToOwned::to_owned))
            };

        let forbidden: fn(&str) -> RequestMiddlewareAction = |content| {
            Response {
                code: StatusCode::FORBIDDEN,
                content_type: HeaderValue::from_static("text/plain; charset=utf-8"),
                content: format!("Authorization error: {}\n", content),
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
            match jsonwebtoken::decode::<Claims>(&token, &self.secret.as_ref()[..], &validation) {
                Ok(data) => data.claims,
                Err(_) => return forbidden("invalid token"),
            };

        let now = self.clock.timestamp();

        // verify 'exp' claim if present.
        // We do not allow any drifting.
        if let Some(exp) = claims.exp {
            if now >= exp {
                return forbidden("token is expired");
            }
        }

        // verify `issued-at` claim
        if claims.iat.is_none() {
            return forbidden("missing issued-at");
        };
        let iat = claims.iat.unwrap();
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

    const SECRET: [u8; 32] = [
        0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
        0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 1u8,
    ];

    fn jwt_handler<C: Clock + Send + Sync + 'static>(clock: C) -> JwtHandler<C> {
        JwtHandler::with_clock(clock, SECRET.into())
    }

    fn assert_respond_with_content(action: RequestMiddlewareAction, expected_content: &str) {
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
                    assert!(
                        content.contains(expected_content),
                        "Invalid content message"
                    )
                }
            }
        }
    }

    #[test]
    fn should_proceed_when_token_is_valid() {
        // given
        let iat = Utc::now().timestamp();
        let claims = Claims {
            iat: Some(iat),
            ..Default::default()
        };
        let header = Header {
            alg: Algorithm::HS256,
            ..Default::default()
        };
        let jwt = encode(&header, &claims, &SECRET).expect("encoding failed.");
        let request = hyper::Request::get("example.com")
            .header("authorization", format!("Bearer {}", jwt))
            .body(Body::empty())
            .expect("request initialization failed");

        // when
        let action = jwt_handler(Utc).on_request(request);

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
        let action = jwt_handler(Utc).on_request(request);

        // then
        assert_respond_with_content(action, "missing token");
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
        let action = jwt_handler(Utc).on_request(request);

        // then
        assert_respond_with_content(action, "invalid token");
    }

    #[test]
    fn should_respond_with_invalid_token_when_invalid_algorithm_used() {
        // given
        let iat = Utc::now().timestamp();
        let claims = Claims {
            iat: Some(iat),
            ..Default::default()
        };
        let header = Header {
            alg: Algorithm::HS512,
            ..Default::default()
        };
        let jwt = encode(&header, &claims, &SECRET).expect("encoding failed.");
        let request = hyper::Request::get("example.com")
            .header("authorization", format!("Bearer {}", jwt))
            .body(Body::empty())
            .expect("request initialization failed");

        // when
        let action = jwt_handler(Utc).on_request(request);

        // then
        assert_respond_with_content(action, "invalid token");
    }

    #[test]
    fn should_respond_with_missing_issued_at_when_iat_is_missing() {
        // given
        let claims = Claims {
            iat: None,
            ..Default::default()
        };
        let header = Header {
            alg: Algorithm::HS256,
            ..Default::default()
        };
        let jwt = encode(&header, &claims, &SECRET).expect("encoding failed.");
        let request = hyper::Request::get("example.com")
            .header("authorization", format!("Bearer {}", jwt))
            .body(Body::empty())
            .expect("request initialization failed");

        // when
        let action = jwt_handler(Utc).on_request(request);

        // then
        assert_respond_with_content(action, "missing issued-at");
    }

    #[test]
    fn should_respond_with_stale_token_when_iat_is_too_old() {
        // given
        let iat = Utc::now().timestamp() - (IAT_WINDOW_SEC + 1);
        let claims = Claims {
            iat: Some(iat),
            ..Default::default()
        };
        let header = Header {
            alg: Algorithm::HS256,
            ..Default::default()
        };
        let jwt = encode(&header, &claims, &SECRET).expect("encoding failed.");
        let request = hyper::Request::get("example.com")
            .header("authorization", format!("Bearer {}", jwt))
            .body(Body::empty())
            .expect("request initialization failed");

        // when
        let action = jwt_handler(Utc).on_request(request);

        // then
        assert_respond_with_content(action, "stale token");
    }

    #[test]
    fn should_respond_with_future_token_when_iat_is_in_future() {
        // given
        let iat = Utc::now().timestamp() + (IAT_WINDOW_SEC + 2);
        let claims = Claims {
            iat: Some(iat),
            ..Default::default()
        };
        let header = Header {
            alg: Algorithm::HS256,
            ..Default::default()
        };
        let jwt = encode(&header, &claims, &SECRET).expect("encoding failed.");
        let request = hyper::Request::get("example.com")
            .header("authorization", format!("Bearer {}", jwt))
            .body(Body::empty())
            .expect("request initialization failed");

        // when
        let action = jwt_handler(Utc).on_request(request);

        // then
        assert_respond_with_content(action, "future token");
    }

    #[test]
    fn should_respond_with_token_is_expired_when_exp_is_too_old() {
        // given
        let now = Utc::now().timestamp();
        let claims = Claims {
            iat: Some(now),
            exp: Some(now - 1),
        };
        let header = Header {
            alg: Algorithm::HS256,
            ..Default::default()
        };
        let jwt = encode(&header, &claims, &SECRET).expect("encoding failed.");
        let request = hyper::Request::get("example.com")
            .header("authorization", format!("Bearer {}", jwt))
            .body(Body::empty())
            .expect("request initialization failed");

        // when
        let action = jwt_handler(Utc).on_request(request);

        // then
        assert_respond_with_content(action, "token is expired");
    }
}
