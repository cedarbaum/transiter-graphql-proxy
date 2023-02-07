use crate::transiter_public::Alert;
use futures::future::join_all;
use lambda_runtime::service_fn;
use nearby_train_times::NearbyTrainTimesNearbyTrainTimesStopRouteTrips;
use nearby_train_times::NearbyTrainTimesNearbyTrainTimesStopRouteTripsRouteTrips;
use nearby_train_times::NearbyTrainTimesNearbyTrainTimesStopRouteTripsRouteTripsTrips;
use route_statuses::RouteStatusesRouteStatuses;
use route_statuses::RouteStatusesRouteStatusesAlerts;
use route_statuses::RouteStatusesRouteStatusesAlertsMessages;
use std::collections::HashMap;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::iter::FromIterator;

use transiter::transiter_public::Route;
use transiter::transiter_public::StopTime;

use graphql_client::GraphQLQuery;
use itertools::Itertools;
use lambda_runtime::{Error, LambdaEvent};
use log::LevelFilter;
use nearby_train_times::NearbyTrainTimesNearbyTrainTimes;
use serde::Serialize;
use serde_json::Value;
use simple_logger::SimpleLogger;

use crate::nearby_train_times::Direction;
use crate::nearby_train_times::NearbyTrainTimesNearbyTrainTimesStopRouteTripsStop;
use crate::route_statuses::RouteStatusesRouteStatusesAlertsMessagesDescriptions;
use crate::route_statuses::RouteStatusesRouteStatusesAlertsMessagesHeaders;
use crate::route_statuses::RouteStatusesRouteStatusesAlertsMessagesUrls;
use crate::transiter::transiter_public;

mod transiter;

#[macro_use]
extern crate lazy_static;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../graphql/schema.graphql",
    query_path = "../../graphql/queries.graphql",
    response_derives = "Debug,Serialize,Clone"
)]
struct NearbyTrainTimes;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../graphql/schema.graphql",
    query_path = "../../graphql/queries.graphql",
    response_derives = "Debug,Serialize,Clone"
)]
struct RouteStatuses;

#[derive(Debug, Serialize)]
pub enum GraphQLError {
    UnknownFieldName(String),
}

impl std::error::Error for GraphQLError {}

impl std::fmt::Display for GraphQLError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GraphQLError::UnknownFieldName(field_name) => {
                write!(f, "Unknown fieldName: {}", field_name)
            }
        }
    }
}

impl TryFrom<&str> for Direction {
    type Error = &'static str;

    fn try_from(direction_str: &str) -> Result<Self, Self::Error> {
        match &direction_str.to_uppercase()[..] {
            "N" | "NORTH" => Ok(Direction::NORTH),
            "S" | "SOUTH" => Ok(Direction::SOUTH),
            _ => Err("Couldn't parse direction."),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let func = service_fn(handler);
    lambda_runtime::run(func).await?;

    Ok(())
}

async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let (event, _context) = event.into_parts();
    let field_name = event["info"]["fieldName"].as_str().unwrap();
    let arguments = &event["arguments"];
    let selection_set = event["info"]["selectionSetGraphQL"].as_str().unwrap();

    match field_name {
        "nearbyTrainTimes" => {
            let lat = arguments["lat"].as_f64().expect("Lat not provided");
            let lon = arguments["lon"].as_f64().expect("Lon not provided");

            // ~2 miles
            let max_distance = 3.2;

            let requested_routes: HashSet<&str> = HashSet::from_iter(
                arguments["routes"]
                    .as_array()
                    .expect("Routes not provided")
                    .iter()
                    .map(|route| route.as_str().unwrap()),
            );
            let direction = Direction::try_from(
                arguments["direction"]
                    .as_str()
                    .expect("Direction not provided"),
            )?;

            get_nearby_train_times(lat, lon, max_distance, &requested_routes, &direction).await
        }
        "routeStatuses" => {
            let requested_routes_array = arguments["routes"].as_array();
            let requested_routes: Option<HashSet<&str>> = if requested_routes_array.is_some() {
                let request_routes_set = HashSet::from_iter(
                    requested_routes_array
                        .unwrap()
                        .iter()
                        .map(|route| route.as_str().unwrap()),
                );
                Some(request_routes_set)
            } else {
                None
            };
            let include_is_running = selection_set.contains("running");

            get_route_statuses(requested_routes.as_ref(), include_is_running).await
        }
        _ => Err(Box::new(GraphQLError::UnknownFieldName(
            field_name.to_string(),
        ))),
    }
}

async fn get_route_statuses(
    requested_routes: Option<&HashSet<&str>>,
    include_is_running: bool,
) -> Result<Value, Error> {
    let all_routes = transiter::get_routes().await?;
    let routes: Vec<&Route> = all_routes
        .routes
        .iter()
        .filter(|route| {
            requested_routes.is_none() || requested_routes.unwrap().contains(&route.id[..])
        })
        .collect();

    let alerts = transiter::get_alerts().await?.alerts;
    let alerts_by_id: HashMap<&String, &Alert> =
        alerts.iter().map(|alert| (&alert.id, alert)).collect();

    let alerts_for_route_ids: HashMap<&String, Vec<RouteStatusesRouteStatusesAlerts>> = routes
        .iter()
        .map(|route| {
            (
                &route.id,
                route
                    .alerts
                    .iter()
                    .map(|alert_preview| {
                        let alert = alerts_by_id.get(&alert_preview.id);
                        let headers = &alert.unwrap().header;
                        let descriptions = &alert.unwrap().description;
                        let urls = &alert.unwrap().url;

                        RouteStatusesRouteStatusesAlerts {
                            cause: Some(alert_preview.cause.clone()),
                            effect: Some(alert_preview.effect.clone()),
                            id: Some(alert_preview.id.clone()),
                            messages: RouteStatusesRouteStatusesAlertsMessages {
                                headers: headers
                                    .iter()
                                    .map(|header| RouteStatusesRouteStatusesAlertsMessagesHeaders {
                                        text: header.text.clone(),
                                        language: header.language.clone(),
                                    })
                                    .collect(),
                                descriptions: descriptions
                                    .iter()
                                    .map(|desc| {
                                        RouteStatusesRouteStatusesAlertsMessagesDescriptions {
                                            text: desc.text.clone(),
                                            language: desc.language.clone(),
                                        }
                                    })
                                    .collect(),
                                urls: urls
                                    .iter()
                                    .map(|url| RouteStatusesRouteStatusesAlertsMessagesUrls {
                                        text: url.text.clone(),
                                        language: url.language.clone(),
                                    })
                                    .collect(),
                            },
                        }
                    })
                    .collect(),
            )
        })
        .collect();

    let route_running_statuses: HashMap<&String, bool> = if include_is_running {
        join_all(
            routes
                .iter()
                .map(|route| get_running_status_for_route(&route.id)),
        )
        .await
        .into_iter()
        .collect::<Result<HashMap<&String, bool>, Error>>()?
    } else {
        HashMap::new()
    };

    let resp = routes
        .iter()
        .map(|route| RouteStatusesRouteStatuses {
            route_id: route.id.clone(),
            running: *route_running_statuses.get(&route.id).unwrap_or(&false),
            alerts: alerts_for_route_ids
                .get(&route.id)
                .unwrap_or(&Vec::new())
                .to_vec(),
        })
        .collect::<Vec<RouteStatusesRouteStatuses>>();

    match serde_json::to_value(resp) {
        Ok(data) => Ok(data),
        Err(error) => Err(Box::new(error)),
    }
}

async fn get_running_status_for_route(route: &String) -> Result<(&String, bool), Error> {
    let trips = transiter::get_route_trips(route).await?.trips;
    Ok((route, trips.len() > 0))
}

async fn get_nearby_train_times(
    lat: f64,
    lon: f64,
    max_distance: f64,
    requested_routes: &HashSet<&str>,
    direction: &Direction,
) -> Result<Value, Error> {
    let nearby_stops = transiter::get_stops(lat, lon, max_distance).await?.stops;
    let stop_id_suffix = match direction {
        Direction::NORTH => "N",
        Direction::SOUTH => "S",
        Direction::Other(_) => panic!("Invalid direction"),
    };

    let nearby_stops_for_direction: Vec<&transiter_public::Stop> = nearby_stops
        .iter()
        .filter(|stop| stop.id.ends_with(stop_id_suffix))
        .collect();

    let stop_distance_by_id: HashMap<&String, f64> = nearby_stops_for_direction
        .iter()
        .map(|stop| {
            (
                &stop.id,
                haversine::distance(
                    haversine::Location {
                        latitude: lat,
                        longitude: lon,
                    },
                    haversine::Location {
                        latitude: stop.latitude.unwrap(),
                        longitude: stop.longitude.unwrap(),
                    },
                    haversine::Units::Kilometers,
                ),
            )
        })
        .collect();

    let resp = NearbyTrainTimesNearbyTrainTimes {
        stop_route_trips: nearby_stops_for_direction
            .iter()
            .map(|stop| NearbyTrainTimesNearbyTrainTimesStopRouteTrips {
                route_trips: get_trips_by_route_for_stop(&stop, requested_routes),
                stop: NearbyTrainTimesNearbyTrainTimesStopRouteTripsStop {
                    distance: (*stop_distance_by_id.get(&stop.id).unwrap()) as f64,
                    stop_id: stop.id.clone(),
                    name: stop.name.clone().unwrap(),
                },
            })
            .filter(|stop_route_trips| stop_route_trips.route_trips.len() > 0)
            .sorted_by(|srt1, srt2| srt1.stop.distance.partial_cmp(&srt2.stop.distance).unwrap())
            .collect::<Vec<NearbyTrainTimesNearbyTrainTimesStopRouteTrips>>(),
        updated_at: None,
    };

    match serde_json::to_value(resp) {
        Ok(data) => Ok(data),
        Err(error) => Err(Box::new(error)),
    }
}

fn get_trips_by_route_for_stop(
    stop: &transiter_public::Stop,
    routes: &HashSet<&str>,
) -> Vec<NearbyTrainTimesNearbyTrainTimesStopRouteTripsRouteTrips> {
    let get_route_id_key = |stop_time: &StopTime| -> String {
        let trip = stop_time.trip.as_ref().unwrap();
        trip.route.as_ref().unwrap().id.clone()
    };

    stop.stop_times
        .iter()
        .filter(|stop_time| stop_time.trip.is_some())
        .sorted_by_key(|stop_time| get_route_id_key(&stop_time))
        .group_by(|stop_time| get_route_id_key(&stop_time))
        .into_iter()
        .filter(|(route, _)| routes.contains(&route[..]))
        .map(
            |(route, stop_times)| NearbyTrainTimesNearbyTrainTimesStopRouteTripsRouteTrips {
                route: route.to_string(),
                trips: stop_times
                    .filter(|stop_time| {
                        (stop_time.arrival.is_some()
                            && stop_time.arrival.as_ref().unwrap().time.is_some())
                            || (stop_time.departure.is_some()
                                && stop_time.departure.as_ref().unwrap().time.is_some())
                    })
                    .map(|stop_time| {
                        NearbyTrainTimesNearbyTrainTimesStopRouteTripsRouteTripsTrips {
                            arrival: stop_time
                                .arrival
                                .as_ref()
                                .unwrap()
                                .time
                                .as_ref()
                                .unwrap_or_else(|| {
                                    &stop_time.departure.as_ref().unwrap().time.as_ref().unwrap()
                                })
                                .parse::<f64>()
                                .unwrap(),
                            trip_id: stop_time.trip.as_ref().unwrap().id.clone(),
                        }
                    })
                    .collect::<Vec<NearbyTrainTimesNearbyTrainTimesStopRouteTripsRouteTripsTrips>>(
                    ),
            },
        )
        .filter(|route_stop_times| route_stop_times.trips.len() > 0)
        .collect()
}
