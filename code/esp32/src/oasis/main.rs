use embassy_executor::Spawner;
use embassy_futures::select;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use embedded_svc::http::{Headers, Method, client::Client as HttpClient};
use embedded_svc::io::{Read, Write};
use embedded_svc::utils::io;
use esp_idf_svc::fs::littlefs::Littlefs;
use esp_idf_svc::hal::task::block_on;
use esp_idf_svc::hal::{gpio, peripherals::Peripherals};
use esp_idf_svc::http::client::EspHttpConnection;
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::io::vfs::MountedLittlefs;
use esp_idf_svc::mdns;
use esp_idf_svc::sntp;
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::wifi;
use esp_idf_svc::wifi::{AsyncWifi, EspWifi};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use rand::Rng;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use terralib::cancel_context::CancelContext;
use terralib::config::{TerrariumConfig, TerrariumConfigUpdate, Update, WifiDetails};
use terralib::controller::{TerrariumController, spin_lock_mutex, terrarium_controller_main_loop};
use terralib::influxdb;
use terralib::terrarium::{get_terrarium_state, print_terrarium_info};
use terralib::types::{ActuatorOverrideSet, SensorValues, TerrariumState};
use terrarium::effects;
use terrarium::real_terrarium::RealTerrarium;

const INDEX_HTML: &str = include_str!("index.html");

const CONFIG_FILE_PATH: &str = "/oasisdata/config.json";

// Need lots of stack to parse JSON
const HTTP_SERVER_STACK_SIZE: usize = 12240;

// Max payload length
const MAX_REQUEST_LEN: usize = 512;

// this channel sends wifi config changes to the wifi management task, which
// asynchronously handles connecting and setting up access point mode.
static WIFI_DETAILS_CHANNEL: Channel<CriticalSectionRawMutex, Option<WifiDetails>, 1> =
    Channel::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Initialize logging to stdout.
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Oasis starting...");

    let peripherals = Peripherals::take().expect("Peripheral init should succeed");

    // setup status led
    let mut status_led =
        gpio::PinDriver::output(peripherals.pins.gpio7).expect("status_led should initialize");
    status_led
        .set_low()
        .expect("driving status led should work");

    // Mount /oasisdata partition and read config file.
    log::info!("Mounting littlefs on /oasisdata partition");
    let _mounted_littlefs = mount_data_partition().expect("Mounting data partition");
    let cfg = match read_config_file() {
        Ok(cfg) => {
            log::info!("Successfully read config file from nvs: {cfg:?}");
            cfg
        }
        Err(err) => {
            // TODO: differentiate between "not found" and other errors
            log::warn!("Unable to read config file from nvs: {err}");
            log::warn!("Using default config");
            let default = TerrariumConfig::new_with_reasonable_defaults();
            log::info!("Writing default config file to flash memory...");
            write_config_file(&default).expect("Saving config file should succeed");
            default
        }
    };

    let terrarium = match RealTerrarium::new(
        peripherals.ledc,
        peripherals.pins.gpio0,
        peripherals.pins.gpio1,
        peripherals.pins.gpio4,
        peripherals.pins.gpio3,
        peripherals.pins.gpio10,
        peripherals.i2c0,
        peripherals.temp_sensor,
    ) {
        Ok(t) => t,
        Err(err) => {
            panic!("Failed to initialize terrarium: {err}");
        }
    };

    let sys_loop = EspSystemEventLoop::take().expect("EspSystemEventLoop should initialize");
    let timer_service = EspTaskTimerService::new().expect("EspTaskTimerService should initialize");
    // note: the nvs partition is used to store wifi calibration data
    let nvs = EspDefaultNvsPartition::take().expect("EspDefaultNvsPartition should initialize");
    let wifi = Arc::new(Mutex::new(
        AsyncWifi::wrap(
            EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))
                .expect("EspWifi should initialize"),
            sys_loop,
            timer_service,
        )
        .expect("AsyncWifi should initialize"),
    ));

    let hostname = cfg.name.clone().unwrap_or("oasis".to_string());
    let cfg_wifi_details = cfg.wifi.clone();

    let controller = Arc::new(Mutex::new(TerrariumController::new(
        Arc::new(Mutex::new(terrarium)),
        cfg,
    )));

    // Initialize mdns service to broadcast the terrarium's hostname on the network.
    let mdns = Arc::new(Mutex::new(
        mdns::EspMdns::take().expect("EspMdns service should initialize"),
    ));
    {
        let mut mdns = mdns.lock().unwrap();
        mdns.set_hostname(hostname.as_str())
            .expect("Setting mdns hostname should succeed");
        mdns.add_service(
            /*instance_name*/ None,
            /*service_type*/
            "_oasis_terrarium",
            /*proto*/
            "_tcp",
            /*port*/ 80,
            /*txt*/ &[],
        )
        .expect("mdns service registration should succeed");
    }
    log::info!("Setup mdns broadcast for '{}'", hostname);

    // Initialize SNTP service to determine current time from the ntp servers on
    // the internet.
    let _sntp = sntp::EspSntp::new_default().expect("SNTP Service should initialize");
    log::info!("SNTP initialized");

    // Sleep for a bit to allow ntp to sync and get our system time accurate.
    // TODO: this makes startup significantly slower and probably isn't necessary.
    Timer::after(Duration::from_secs(5)).await;

    // Setup http server
    let server_configuration = esp_idf_svc::http::server::Configuration {
        stack_size: HTTP_SERVER_STACK_SIZE,
        ..Default::default()
    };
    let mut http_server =
        EspHttpServer::new(&server_configuration).expect("Http server should initialize");

    // The "/" route returns the main html ui page
    http_server
        .fn_handler("/", Method::Get, |req| {
            req.into_ok_response()?
                .write_all(INDEX_HTML.as_bytes())
                .map(|_| ())
        })
        .expect("Http handler registration should succeed");

    // The "/control" route accepts POST requests that specify values to set the
    // lights, fan, and mister to.
    let ctlref1 = controller.clone();
    http_server
        .fn_handler::<anyhow::Error, _>("/control", Method::Post, move |mut req| {
            let len = req.content_len().unwrap_or(0) as usize;

            if len > MAX_REQUEST_LEN {
                req.into_status_response(413)?
                    .write_all("Request too big".as_bytes())?;
                return Ok(());
            }

            let mut buf = vec![0; len];
            req.read_exact(&mut buf)?;

            let update_data = match serde_json::from_slice::<ActuatorOverrideSet>(&buf) {
                Err(e) => {
                    req.into_status_response(413)?
                        .write_all(format!("json parse error: '{e}'").as_bytes())?;
                    return Ok(());
                }
                Ok(d) => d,
            };

            match ctlref1.lock().unwrap().handle_control_cmd(&update_data) {
                Ok(()) => {}
                Err(err) => {
                    // TODO: use more specific response codes as needed
                    req.into_status_response(400)?
                        .write_all(err.to_string().as_bytes())?;
                }
            }

            Ok(())
        })
        .expect("Http handler registration should succeed");

    // The "/state" route returns the current actuator settings and sensor
    // readings.
    let ctlref2 = controller.clone();
    http_server
        .fn_handler::<anyhow::Error, _>("/state", Method::Get, move |req| {
            let mut resp = req.into_ok_response()?;

            // TODO: this can be done better. write bytes directly to resp
            // rather than creating a vec then converting.
            let js = serde_json::json!(get_terrarium_state(
                &mut *ctlref2.lock().unwrap().terrarium().lock().unwrap()
            ));
            let mut bytes: Vec<u8> = Vec::new();
            serde_json::to_writer(&mut bytes, &js).unwrap();
            resp.write(bytes.as_slice())?;

            Ok(())
        })
        .expect("Http handler registration should succeed");

    // GET "/config" returns the terrarium configuration
    let ctlref3 = controller.clone();
    http_server
        .fn_handler::<anyhow::Error, _>("/config", Method::Get, move |req| {
            let mut resp = req.into_ok_response()?;

            let js = serde_json::json!(*ctlref3.lock().unwrap().config());
            let mut bytes: Vec<u8> = Vec::new();
            serde_json::to_writer(&mut bytes, &js).unwrap();
            resp.write(bytes.as_slice())?;

            Ok(())
        })
        .expect("Http handler registration should succeed");

    let ctlref4 = controller.clone();
    http_server
        .fn_handler::<anyhow::Error, _>("/config", Method::Post, move |mut req| {
            log::info!("got POST /config");
            let len = req.content_len().unwrap_or(0) as usize;

            if len > MAX_REQUEST_LEN {
                log::error!("/config request body too big");
                req.into_status_response(413)?
                    .write_all("Request too big".as_bytes())?;
                return Ok(());
            }

            let mut buf = vec![0; len];
            req.read_exact(&mut buf)?;

            let cfg_update = match serde_json::from_slice::<TerrariumConfigUpdate>(&buf) {
                Ok(d) => d,
                Err(e) => {
                    req.into_status_response(413)?
                        .write_all(format!("json parse error: '{e}'").as_bytes())?;
                    return Ok(());
                }
            };

            // See if hostname or wifi creds changed
            // TODO: compare values of update.wifi and old config.wifi. It might
            // be a Set, but with the same value as before.
            let needs_wifi_reset = cfg_update.wifi != Update::NoChange;
            let needs_hostname_change = cfg_update.name != Update::NoChange;

            if let Err(err) = ctlref4.lock().unwrap().update_config(&cfg_update) {
                log::error!("Error updating config: {}", err);
                req.into_status_response(413)?
                    .write_all(format!("config update rejected: {err}").as_bytes())?;
                return Ok(());
            }
            write_config_file(ctlref4.lock().unwrap().config())?;

            log::info!("Successfully updated config via /config http");

            if needs_wifi_reset {
                // send new wifi details to wifi management task
                block_on(send_wifi_details(
                    ctlref4.lock().unwrap().config().wifi.clone(),
                ));
            }
            if needs_hostname_change {
                let hostname = ctlref4
                    .lock()
                    .unwrap()
                    .config()
                    .name
                    .clone()
                    .unwrap_or("oasis".to_string());
                mdns.lock()
                    .unwrap()
                    .set_hostname(&hostname)
                    .expect("Setting mdns hostname should succeed");
                log::info!("Successfully updated mdns hostname to '{}'", hostname)
            }

            Ok(())
        })
        .expect("Http handler registration should succeed");

    print_terrarium_info(
        &mut *spin_lock_mutex(&*controller)
            .await
            .terrarium()
            .lock()
            .unwrap(),
    );

    // lightning test
    if false {
        controller
            .lock()
            .unwrap()
            .terrarium()
            .lock()
            .unwrap()
            .set_lights(1.0);
        spawner.must_spawn(effects::lightning(
            spin_lock_mutex(&*controller).await.terrarium(),
        ));
    }

    spawner.must_spawn(terrarium_controller_main_loop(controller.clone()));
    spawner.must_spawn(record_to_influxdb_forever(controller.clone()));
    spawner.must_spawn(reset_button_watcher(
        peripherals.pins.gpio9,
        controller.clone(),
        spawner,
    ));
    spawner.must_spawn(wifi_management_task(wifi));

    // send initial wifi info based on the config file.
    block_on(send_wifi_details(cfg_wifi_details));

    // wait forever
    loop {
        Timer::after(Duration::from_secs(100000)).await;
    }
}

#[embassy_executor::task]
async fn reset_button_watcher(
    pin: gpio::Gpio9,
    controller: Arc<Mutex<TerrariumController>>,
    spawner: Spawner,
) {
    // button pulls low via 10k R when pressed
    // Some of this code comes from:
    // https://github.com/esp-rs/std-training/blob/5831eba5c7735400580a2e35116b87834f714a13/advanced/button-interrupt/examples/solution.rs
    let mut rst_button = gpio::PinDriver::input(pin).expect("reset pin");
    rst_button.set_pull(gpio::Pull::Up).expect("reset pin");

    loop {
        rst_button.wait_for_falling_edge().await.unwrap();
        println!("Reset button pressed!");

        // Start breathing leds to indicate that the reset button press was/is registered.
        spin_lock_mutex(&*controller).await.takeover_lights();
        let breathe_ctx = Arc::new(CancelContext::new());
        spawner.must_spawn(effects::breathe(
            spin_lock_mutex(&*controller).await.terrarium(),
            0.05,
            0.5,
            1_000,
            breathe_ctx.clone(),
        ));

        // Wait until either 5 seconds has elapsed (then do a reset) or the
        // button is released early (reset is cancelled).
        match select::select(
            rst_button.wait_for_rising_edge(),
            Timer::after(Duration::from_secs(5)),
        )
        .await
        {
            select::Either::First(_) => {
                // button was released early, don't reset
                // cancel breathe effect
                breathe_ctx.cancel_and_wait().await;
                spin_lock_mutex(&*controller).await.release_lights();
                log::info!("Rst button was released early, not resetting");
            }
            select::Either::Second(_) => {
                // button was held down for 5 seconds, do a reset

                // cancel breathe effect
                breathe_ctx.cancel_and_wait().await;
                spin_lock_mutex(&*controller).await.release_lights();
                log::info!("Rst button was held down for 10s, resetting...");

                // do a second "breathe" effect, but blink much faster to
                // indicate that reset was registered.
                let breathe_ctx2 = Arc::new(CancelContext::new());
                spawner.must_spawn(effects::breathe(
                    spin_lock_mutex(&*controller).await.terrarium(),
                    0.05,
                    0.5,
                    100,
                    breathe_ctx2.clone(),
                ));
                Timer::after(Duration::from_secs(1)).await;
                breathe_ctx2.cancel_and_wait().await;

                // Delete config file and reboot.
                if let Err(err) = delete_config_file() {
                    log::error!("Error deleting config file: {err}");
                }
                log::error!("Restarting esp32");
                esp_idf_hal::reset::restart();
            }
        }
    }
}

// Sends new wifi details to a channel, which get picked up by the wifi
// management task. This function exists because `await` can't be called from
// http handlers, which aren't async.
async fn send_wifi_details(wifi_details: Option<WifiDetails>) {
    WIFI_DETAILS_CHANNEL.send(wifi_details).await;
}

// If the given wifi_details is Some, connect to the specified network. If
// network connection fails or if no network is specified, setup an access
// point named something like "oasis-xxxx" that the user can connect to.
async fn try_connect_wifi_with_ap_fallback(
    wifi: &mut AsyncWifi<EspWifi<'static>>,
    wifi_details_opt: &Option<WifiDetails>,
) {
    // Try to connect to network if specified
    if let Some(wifi_details) = &wifi_details_opt {
        log::info!("Connecting to wifi using creds from config...");
        if let Err(connect_err) = connect_wifi(wifi, &wifi_details).await {
            log::error!(
                "Error connecting to wifi network '{}': {}",
                wifi_details.ssid,
                connect_err
            );
        } else {
            // Success! We're on the network.
            log::info!("Connected to wifi network '{}'", wifi_details.ssid);
            match wifi.wifi().sta_netif().get_ip_info() {
                Ok(ip_info) => {
                    log::info!("Terrarium is live at ip address: {}", ip_info.ip)
                }
                Err(err) => log::error!("Error getting ip address: {}", err),
            }
            return;
        }
    }

    // If no network was specified or we failed to connect,
    // setup/broadcast our own access point.
    log::error!("Setting up access point...");
    if let Err(err) = setup_wifi_ap(wifi).await {
        log::error!("Error setting up access point: {}", err);
        // TODO: what do we do here? setting up an access point shouldn't fail -
        // it doesn't have any dependencies external to the device. do we
        // reboot? what if we get stuck in a reboot loop?
    } else {
        log::info!("Access point setup");
    }
}

// Subscribe to a channel that passes WifiDetails on each change. Setup an
// access point or network connection based on them. If connect fails, setup an
// access point, then periodically retry the connection to the specified
// network in case it comes back online.
#[embassy_executor::task]
async fn wifi_management_task(wifi: Arc<Mutex<AsyncWifi<EspWifi<'static>>>>) {
    let mut latest_wifi_setup_time = Instant::now();
    let mut latest_wifi_details: Option<WifiDetails> = None;

    loop {
        match select::select(
            WIFI_DETAILS_CHANNEL.receive(),
            Timer::after(Duration::from_secs(61)),
        )
        .await
        {
            select::Either::First(wifi_details_opt) => {
                log::info!(
                    "wifi_management_task got new wifi details: {:?}",
                    wifi_details_opt
                );
                latest_wifi_details = wifi_details_opt;

                // WifiDetails typically come in on the channel from an http
                // handler. If we immediately reconfigure wifi, the http
                // handler won't be able to send a response. Wait a short
                // period to allow the handler to finish, then re-configure
                // wifi.
                Timer::after(Duration::from_millis(10)).await;

                let mut wifi = wifi.lock().unwrap();
                try_connect_wifi_with_ap_fallback(&mut wifi, &latest_wifi_details).await;
                latest_wifi_setup_time = Instant::now();
            }
            select::Either::Second(_) => {
                if latest_wifi_details.is_some() {
                    let mut wifi = wifi.lock().unwrap();
                    // "ap mode" means the terrarium is hosting/broadcasting
                    //  it's own wifi network. It does this temporarily if we
                    //  have credentials to connect to another network, but we
                    //  failed to connect.
                    let in_temporary_ap_mode = match wifi.wifi().driver().is_ap_started() {
                        Ok(is_ap_started) => is_ap_started,
                        Err(err) => {
                            log::error!("Error querying wifi mode: {}", err);
                            false
                        }
                    };
                    if in_temporary_ap_mode {
                        let timeout_elapsed = (Instant::now() - latest_wifi_setup_time)
                            > std::time::Duration::from_secs(5 * 60);
                        if timeout_elapsed {
                            try_connect_wifi_with_ap_fallback(&mut wifi, &latest_wifi_details)
                                .await;
                            latest_wifi_setup_time = Instant::now();
                        }
                    } else {
                        let wifi_still_connected = match wifi.is_connected() {
                            Ok(is_connected) => is_connected,
                            Err(err) => {
                                log::error!("Error getting wifi connection status: {}", err);
                                false
                            }
                        };

                        if !wifi_still_connected {
                            // wifi connection got dropped, try again
                            try_connect_wifi_with_ap_fallback(&mut wifi, &latest_wifi_details)
                                .await;
                            latest_wifi_setup_time = Instant::now();
                        }
                    }
                }
            }
        };
    }
}

// Create our own wifi network named "oasis-xxxx" (where xxxx is a random 4-digit number)
async fn setup_wifi_ap(wifi: &mut AsyncWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    if wifi.is_started()? {
        log::error!("Stopping wifi...");
        wifi.stop().await?;
    }

    // Generate an access point ssid with a random 4-digit suffix
    let mut rng = rand::rng();
    let suffix = rng.random_range(1000..=9999);
    let mut ap_ssid: heapless::String<32> = heapless::String::new();
    std::fmt::write(&mut ap_ssid, format_args!("oasis-{suffix}")).expect("");

    let wifi_configuration = wifi::Configuration::AccessPoint(wifi::AccessPointConfiguration {
        ssid: ap_ssid.clone(),
        password: "".try_into().unwrap(),
        auth_method: wifi::AuthMethod::None,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;
    wifi.start().await?;

    log::info!("WiFi AP '{ap_ssid}' started successfully!");

    Ok(())
}

// Connect to the wifi network
async fn connect_wifi(
    wifi: &mut AsyncWifi<EspWifi<'static>>,
    creds: &WifiDetails,
) -> anyhow::Result<()> {
    if wifi.is_started()? {
        log::error!("Stopping wifi...");
        wifi.stop().await?;
    }

    let wifi_configuration: wifi::Configuration =
        wifi::Configuration::Client(wifi::ClientConfiguration {
            ssid: creds.ssid.as_str().try_into().expect(""),
            bssid: None,
            auth_method: wifi::AuthMethod::WPA2Personal,
            password: creds.password.as_str().try_into().expect(""),
            channel: None,
            ..Default::default()
        });

    wifi.set_configuration(&wifi_configuration)?;
    wifi.start().await?;
    wifi.connect().await?;
    wifi.wait_netif_up().await?;

    Ok(())
}

fn mount_data_partition() -> anyhow::Result<MountedLittlefs<Littlefs<u8>>> {
    let mut littlefs: Littlefs<u8> = unsafe { Littlefs::new_partition("oasisdata")? };
    let mounted_littlefs = match MountedLittlefs::mount(littlefs, "/oasisdata") {
        Ok(lfs) => lfs,
        Err(err) => {
            log::warn!("Failed to mount littlefs with error: {err}");
            log::warn!(
                "Assuming (blindly) that mount failure was due to lack of littlefs formatting. trying a format"
            );
            log::warn!("Formatting the littlefs partition");
            littlefs = unsafe { Littlefs::new_partition("oasisdata")? };
            littlefs.format()?;
            log::warn!("littlefs partition formatted. attempting to mount again");
            MountedLittlefs::mount(littlefs, "/oasisdata")?
        }
    };
    log::info!("Filesystem usage: {:?}", mounted_littlefs.info()?);
    Ok(mounted_littlefs)
}

fn read_config_file() -> anyhow::Result<TerrariumConfig> {
    let file = File::open(CONFIG_FILE_PATH)?;
    let state: TerrariumConfig = serde_json::from_reader(file)?;
    Ok(state)
}

fn delete_config_file() -> anyhow::Result<()> {
    Ok(std::fs::remove_file(CONFIG_FILE_PATH)?)
}

fn write_config_file(cfg: &TerrariumConfig) -> anyhow::Result<()> {
    let file = File::create(CONFIG_FILE_PATH)?;
    serde_json::to_writer(file, cfg)?;
    Ok(())
}

// Temperature conversion.
fn c_to_f(c: f32) -> f32 {
    c * 1.8 + 32.0
}

// Records the current terrarium state to influxdb. Useful for tracking
// humidity+temperature, etc. over time and drawing pretty graphs.
fn record_to_influxdb(
    client: &mut HttpClient<EspHttpConnection>,
    config: &influxdb::Config,
    state: &TerrariumState,
) -> anyhow::Result<()> {
    let mist = if state.actuators.mist { 1.0 } else { 0.0 };
    let fan = if state.actuators.fans { 1.0 } else { 0.0 };
    let sens_vals = if let Some(sens_vals) = state.sensors {
        sens_vals
    } else {
        SensorValues {
            humid: -1.0,
            temp: -1.0,
        }
    };
    let cpu_temp_f = if let Some(cpu_temp) = state.cpu_temp {
        c_to_f(cpu_temp)
    } else {
        -1.0
    };
    let payload = std::format!(
        "mydata sht30.temperature_c={},sht30.temperature_f={},sht30.humidity={},led.value={},mist.value={},fan.value={},cpu_temp_f.value={}",
        sens_vals.temp,
        c_to_f(sens_vals.temp),
        sens_vals.humid * 100.0,
        state.actuators.lights,
        mist,
        fan,
        cpu_temp_f,
    );

    // Prepare headers and URL
    let auth = std::format!("Token {}", config.token);
    let headers = [
        ("Authorization", auth.as_str()),
        ("Accept", "application/json"),
    ];
    let url = std::format!(
        "{}/api/v2/write?bucket={}&org={}",
        config.address,
        config.bucket,
        config.org,
    );

    // Send request
    let mut request = client.post(&url, &headers)?;
    request.write_all(payload.as_bytes())?;
    request.flush()?;
    let mut response = request.submit()?;

    // Process response
    let status = response.status();
    if status < 200 || status > 300 {
        let mut buf = [0u8; 1024];
        let bytes_read = io::try_read_full(&mut response, &mut buf).map_err(|e| e.0)?;
        return Err(match std::str::from_utf8(&buf[0..bytes_read]) {
            Ok(body_string) => {
                anyhow::anyhow!(
                    "Error from influxdb (truncated to {} bytes): {:?}",
                    buf.len(),
                    body_string
                )
            }
            Err(e) => anyhow::anyhow!("Error decoding response body from influxdb: {e}"),
        });
    }

    Ok(())
}

// Records the terrarium's state to influx db every 10 seconds.
#[embassy_executor::task]
async fn record_to_influxdb_forever(controller: Arc<Mutex<TerrariumController>>) {
    loop {
        Timer::after(Duration::from_secs(10)).await;
        let ctlr = spin_lock_mutex(&*controller).await;
        if let Some(config) = &ctlr.config().influxdb {
            let mut client =
                HttpClient::wrap(EspHttpConnection::new(&Default::default()).expect("default"));
            let locked_terrarium = &*ctlr.terrarium();
            let mut terrarium = spin_lock_mutex(locked_terrarium).await;
            if let Err(err) =
                record_to_influxdb(&mut client, config, &get_terrarium_state(&mut *terrarium))
            {
                log::error!("Error recording to influxdb: {err}");
            }
        }
    }
}
