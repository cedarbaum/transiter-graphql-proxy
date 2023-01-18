use reqwest::{Method, Response};
use serde::de::DeserializeOwned;
use std::env;
use stopwatch::Stopwatch;

pub mod transiter_public {
    include!(concat!(env!("OUT_DIR"), "/transiter.public.rs"));
}

lazy_static! {
    static ref TRANSITER_HOST: String = env::var("TRANSITER_HOST").expect("TRANSITER_HOST not set");
    static ref CLIENT: reqwest::Client = reqwest::Client::new();
}

pub async fn get_stops(
    lat: f64,
    lon: f64,
    distance: f64,
) -> Result<transiter_public::ListStopsReply, reqwest::Error> {
    call_transiter_and_parse_json::<transiter_public::ListStopsReply>(
        Method::GET,
        "stops",
        Some(&[
            ("filter_by_distance", &"true"),
            ("sort_mode", &"DISTANCE"),
            ("latitude", &lat.to_string()),
            ("longitude", &lon.to_string()),
            ("max_distance", &distance.to_string()),
        ]),
    )
    .await
}

pub async fn get_routes() -> Result<transiter_public::ListRoutesReply, reqwest::Error> {
    call_transiter_and_parse_json::<transiter_public::ListRoutesReply>(Method::GET, "routes", None)
        .await
}

pub async fn get_route_trips(
    route_id: &str,
) -> Result<transiter_public::ListTripsReply, reqwest::Error> {
    let path = &format!("routes/{}/trips", route_id)[..];
    call_transiter_and_parse_json::<transiter_public::ListTripsReply>(
        Method::GET,
        path,
        Some(&[("exclude_trips_before", "1")]),
    )
    .await
}

pub async fn get_alerts() -> Result<transiter_public::ListAlertsReply, reqwest::Error> {
    call_transiter_and_parse_json::<transiter_public::ListAlertsReply>(Method::GET, "alerts", None)
        .await
}

pub async fn call_transiter_and_parse_json<T: DeserializeOwned + std::fmt::Debug>(
    method: Method,
    path: &str,
    query_params: Option<&[(&str, &str)]>,
) -> Result<T, reqwest::Error> {
    let response = call_transiter(method, path, query_params).await?;
    response.json::<T>().await
}

pub async fn call_transiter(
    method: Method,
    path: &str,
    query_params: Option<&[(&str, &str)]>,
) -> Result<Response, reqwest::Error> {
    let sw = Stopwatch::start_new();

    let url_str = &format!(
        "http://{}/systems/us-ny-subway/{}",
        &TRANSITER_HOST[..],
        path
    )[..];
    let url = reqwest::Url::parse_with_params(url_str, query_params.unwrap_or(&[]))
        .expect("Failed to parse URL");

    log::info!("{:?}\n", path);
    let request = reqwest::Request::new(method, url);
    let result = CLIENT.execute(request).await;

    log::info!("Calling transiter path {} took {}ms", path, sw.elapsed_ms());
    result
}
