//! The demoserver implements the same http interface as the esp32 program, but
//! runs on linux so some things can be tested without having to run on the
//! esp32.

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::Html,
    routing::{get, post},
};
use std::sync::{Arc, Mutex};
use terralib::config::{TerrariumConfig, TerrariumConfigUpdate, WifiDetails};
use terralib::controller::TerrariumController;
use terralib::terrarium::{FakeTerrarium, get_terrarium_state, print_terrarium_info};
use terralib::types::{ActuatorOverrideSet, TerrariumState};

const INDEX_HTML: &str = include_str!("../../esp32/src/oasis/index.html");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    stderrlog::new()
        .module(module_path!())
        .verbosity(log::Level::Info)
        .init()?;

    log::info!("Starting oasis demoserver...");

    let terrarium = Arc::new(Mutex::new(FakeTerrarium::new()));
    let mut cfg = TerrariumConfig::new_with_reasonable_defaults();
    cfg.wifi = Some(WifiDetails {
        ssid: "ssid".to_string(),
        password: "password".to_string(),
    });
    cfg.name = Some("oasis".to_string());
    let controller = Arc::new(Mutex::new(TerrariumController::new(terrarium, cfg)));

    print_terrarium_info(&mut *controller.lock().unwrap().terrarium().lock().unwrap());

    let app = Router::new()
        .route("/", get(root))
        .route("/state", get(state))
        .route("/control", post(control))
        .route("/config", post(update_config))
        .route("/config", get(get_config))
        .with_state(controller);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    log::info!("Listening on port 3000");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn root() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn state(
    State(controller): State<Arc<Mutex<TerrariumController>>>,
) -> Result<Json<TerrariumState>, (StatusCode, String)> {
    Ok(Json(get_terrarium_state(
        &mut *controller.lock().unwrap().terrarium().lock().unwrap(),
    )))
}

async fn control(
    State(controller): State<Arc<Mutex<TerrariumController>>>,
    Json(cmd): Json<ActuatorOverrideSet>,
) -> StatusCode {
    // TODO: different error codes depending on what happened
    log::info!("/control called with {cmd:?}");
    match controller.lock().unwrap().handle_control_cmd(&cmd) {
        Ok(()) => {}
        Err(err) => {
            // TODO: return the error message in the response body
            log::error!("Error handling /control: {err:?}");
            return StatusCode::BAD_REQUEST;
        }
    }
    StatusCode::OK
}

async fn update_config(
    State(controller): State<Arc<Mutex<TerrariumController>>>,
    Json(cfg_update): Json<TerrariumConfigUpdate>,
) -> StatusCode {
    // TODO: different error codes depending on what happened
    log::info!("POST /config called with {cfg_update:?}");
    match controller.lock().unwrap().update_config(&cfg_update) {
        Ok(()) => {}
        Err(err) => {
            // TODO:  include error message in response body
            log::error!("Error handling /config: {err:?}");
            return StatusCode::BAD_REQUEST;
        }
    }
    StatusCode::OK
}

async fn get_config(
    State(controller): State<Arc<Mutex<TerrariumController>>>,
) -> Result<Json<TerrariumConfig>, (StatusCode, String)> {
    log::info!("GET /config called");
    Ok(Json(controller.lock().unwrap().config().clone()))
}
