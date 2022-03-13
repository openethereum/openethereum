//! Test cases have been adapted from corresponding JWT module in Nethermind.

use jwt_authentication::{Clock, JwtHandler};

use jsonrpc_http_server::{
    hyper::{self, Body},
    RequestMiddleware, RequestMiddlewareAction,
};
use rstest::rstest;

struct MockClock {
    timestamp: i64,
}

impl MockClock {
    fn new(timestamp: i64) -> Self {
        Self { timestamp }
    }
}

impl Clock for MockClock {
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
}

fn is_proceeded(action: RequestMiddlewareAction) -> bool {
    match action {
        RequestMiddlewareAction::Proceed { .. } => true,
        RequestMiddlewareAction::Respond { .. } => false,
    }
}

#[rstest]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzF9.3MaCM_vL7Dl50v0FMEJeVWwYckxifqxGtA2dlZA9YHQ", true)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzV9.QRtFFE5NnbK_mMu-3qtPGPiAgTRCvb-Z1Ti_uwBjgDk", true)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5Njd9.lJP7Nw_Lio-gP78ZW-Uv3PVdLbuaIMVgU9uvLw1V1BY", true)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE2NDQ5OTQ5NzMsImlhdCI6MTY0NDk5NDk3MX0.1RVPaAjpjQWFqm33C87zdUThUbob96C5SHBVn_LDLDc", true)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJiYXIiOiJiYXoiLCJpYXQiOjE2NDQ5OTQ5NzF9.EU7c1vsCWHU9fCV888yf1IwJR7uczhk5pKCB6CAd_NI", true)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5Nzd9.r_MM-6TLGUtsf_EalbJKxgO-Vw6LOkTEqKjcEBSCRHw", false)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NjV9.sWMMjsne2hK0S20OL3lP_qVvnGIGvBc5fa7sUvJUiqM", false)]
#[case("Bearer eyJhbGciOiJIUzUxMiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzV9.Av2ZI-xeXA8-VuSoYxCsnn0cCg_4St2zOSgFKbvsS1ObTZKLeltSV4CcTcraukYL_HNun3rI4iDjDxs6EJgbCA", false)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE2NDQ5OTQ5NzEsImlhdCI6MTY0NDk5NDk3MX0.Nc6fT-W8bknDUqnjEwHKLreTguYgzMBlbsPAMO2OOHM", false)]
#[case(
    "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.t-IDcSemACt8x4iTMCda8Yhe3iZaWbvV5XKSTbuAn0M",
    false
)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzF9.tICF9zHKdMOwccLLA2LGqbA_P1X8WHD-KMe5R4GpgkE", false)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzF9.JxoxCpDIzhNLqBCvSWJjddHQ87SynxgwTjJP0-PapA4", false)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzF9.JxoxCpDIzhNLqBCvSWJjddHQ87SynxgwTjJP0-PapA4", false)]
#[case("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzF9.3MaCM_vL7Dl50v0FMEJeVWwYckxifqxGtA2dlZA9YHQ", false)]
#[case("Bearer  eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzF9.3MaCM_vL7Dl50v0FMEJeVWwYckxifqxGtA2dlZA9YHQ", false)]
#[case("bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzF9.3MaCM_vL7Dl50v0FMEJeVWwYckxifqxGtA2dlZA9YHQ", false)]
#[case("Bearer: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzF9.3MaCM_vL7Dl50v0FMEJeVWwYckxifqxGtA2dlZA9YHQ", false)]
#[case("Bearer:eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzF9.3MaCM_vL7Dl50v0FMEJeVWwYckxifqxGtA2dlZA9YHQ", false)]
#[case("Bearer\teyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzF9.3MaCM_vL7Dl50v0FMEJeVWwYckxifqxGtA2dlZA9YHQ", false)]
#[case("Bearer \teyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzF9.3MaCM_vL7Dl50v0FMEJeVWwYckxifqxGtA2dlZA9YHQ", false)]
fn geth_tests(#[case] token: String, #[case] expected: bool) {
    let clock = MockClock::new(1644994971);
    // Nethermind uses secret of 6 bytes in corresponding test. However, we allow secret
    // to be 32 bytes long only. So, we have to pad it with zeros to get the same results.
    let secret: [u8; 32] = [
        0x73, 0x65, 0x63, 0x72, 0x65, 0x74, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
        0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
    ];
    let jwt_handler = JwtHandler::with_clock(clock, secret.into());

    let request = hyper::Request::get("example.com")
        .header("authorization", token)
        .body(Body::empty())
        .expect("request initialization failed");

    // when
    let action = jwt_handler.on_request(request);
    assert_eq!(expected, is_proceeded(action))
}

#[rstest]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzF9.RmIbZajyYGF9fhAq7A9YrTetdf15ebHIJiSdAhX7PME", true)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzV9.HfWy49SIyB12PBB_xEpy6IAiIan5mIqD6Jzeh_J1QNw", true)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5Njd9.YGA0v88qMS7lp41wJQv9Msru6dwrNOHXHYiDsuhuScU", true)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE2NDQ5OTQ5NzMsImlhdCI6MTY0NDk5NDk3MX0.ADc_b_tCac2uRHcNCekHvHV-qQ8hNyUjdxCVPETd3Os", true)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJiYXIiOiJiYXoiLCJpYXQiOjE2NDQ5OTQ5NzF9.UZmoAYPGvKoWvz3KcXuxkDnVIF4Fn7QT7z9RwZgSREo", true)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5Nzd9.QydUOgQDbnaM66i5-YKWFqmQFV_vqO2-wHCR0GbyUz8", false)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NjV9.PvVSCk5oBSgJ77JNUw_PM9kak-1aM9VJD1qvTNIpFVw", false)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzF9.RmIbZajyYGF9fhAq7A9YrTetdf15ebHIJiSdAhX7PMe", false)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzF9.RmIbZajyYGF9fhAq7A9YrTetdf15ebHIJiSdAhX7PMEe", false)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpYXQiOjE2NDQ5OTQ5NzF8.RmIbZajyYGF9fhAq7A9YrTetdf15ebHIJiSdAhX7PME", false)]
#[case(
    "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.d88KZjmZ_nL0JTnsF6SR1BRBCjus4U3M-390HDDDNRc",
    false
)]
#[case("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE2NDQ5OTQ5NzEsImlhdCI6MTY0NDk5NDk3MX0.wU4z8ROPW-HaOgrUBG0FqTEutt7rWVsWMqXLvdEl_wI", false)]
fn long_key_tests(#[case] token: String, #[case] expected: bool) {
    let clock = MockClock::new(1644994971);
    let secret: [u8; 32] = [
        0x51, 0x66, 0x54, 0x6A, 0x57, 0x6E, 0x5A, 0x72, 0x34, 0x75, 0x37, 0x78, 0x21, 0x41, 0x25,
        0x44, 0x2A, 0x47, 0x2D, 0x4A, 0x61, 0x4E, 0x64, 0x52, 0x67, 0x55, 0x6B, 0x58, 0x70, 0x32,
        0x73, 0x35,
    ];
    let jwt_handler = JwtHandler::with_clock(clock, secret.into());

    let request = hyper::Request::get("example.com")
        .header("authorization", token)
        .body(Body::empty())
        .expect("request initialization failed");

    // when
    let action = jwt_handler.on_request(request);
    assert_eq!(expected, is_proceeded(action))
}
