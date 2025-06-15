// This file defines the command-line client program for the Oasis terrarium.
//
// This program can be used to query the state of the terrarium, configure it's
// schedule, and control its lights, mist, and fans.

use anyhow::anyhow;
use clap::{Parser, Subcommand};
use itertools::Itertools;
use mdns_sd::{ServiceDaemon, ServiceEvent};
use regex::Regex;
use reqwest::StatusCode;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;
use terralib::config::TerrariumConfigUpdate;
use terralib::terrarium::print_terrarium_state;
use terralib::types::{
    Actuator, ActuatorOverride, ActuatorOverrideSet, ActuatorValue, TerrariumState,
};

#[derive(Parser, Debug)]
#[command(about = "Oasis terrarium command-line client")]
struct Args {
    #[arg(long, help = "Hostname or IP address of terrarium")]
    addr: Option<String>,

    #[command(subcommand)]
    command: Commands,

    #[arg(long, help = "If true, output is printed in json format")]
    json: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Control the lights, fans, and mister.
    Ctl {
        #[arg(help = "A list of actuator commands of the form <actuator>@<value>:<duration>")]
        overrides: Vec<String>,
    },
    /// Get the temperature and humidity of the terrarium, as well as the current state of the lights, fans, and mister.
    State,
    /// Get or set the terrarium configuration. This includes wifi details and schedule.
    Config {
        #[arg(long)]
        config_json: Option<String>,
        #[arg(long)]
        config_file: Option<String>,
    },
    /// Scan the local network for online terrariums.
    Scan {
        #[arg(help = "How long to scan mdns for (in seconds)", value_parser = parse_duration, default_value = "10")]
        timeout: Duration,
    },
}

fn parse_duration(arg: &str) -> Result<Duration, std::num::ParseIntError> {
    let seconds = arg.parse()?;
    Ok(Duration::from_secs(seconds))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    stderrlog::new()
        .module(module_path!())
        .verbosity(log::Level::Info)
        .init()
        .unwrap();

    let args = Args::parse();

    // The terrarium address can be passed by flag or environment variable. Flag
    // takes precedence.
    let addr = args
        .addr
        .or(env::var("OASIS_ADDR").ok())
        .expect("No address specified. Either pass --addr or set OASIS_ADDR env var.");

    match &args.command {
        Commands::Ctl { overrides } => {
            log::info!("Connecting to terrarium at '{addr}'...");
            let client = reqwest::Client::new();

            let cmds: Result<Vec<ControlCommand>, CommandParseError> =
                overrides.iter().map(|x| parse_cmd(x)).collect();
            match cmds {
                Ok(_) => {}
                Err(ref err) => {
                    panic!("Error parsing command(s): {err}");
                }
            }

            let update_data = create_update_data(&cmds.unwrap());
            let control_uri = format!("http://{addr}/control");
            let resp = client.post(control_uri).json(&update_data).send().await?;
            if resp.status() != StatusCode::OK {
                return Err(anyhow!("Control failed: {}", resp.text().await?));
            }
        }
        Commands::State => {
            log::info!("Connecting to terrarium at '{addr}'...");
            let client = reqwest::Client::new();

            let state_uri = format!("http://{addr}/state");
            let resp = client.get(state_uri).send().await?;
            if resp.status() != StatusCode::OK {
                return Err(anyhow!(
                    "Got bad response: {}",
                    resp.text().await.expect("resp text")
                ));
            }

            let text = resp.text().await.unwrap();
            if args.json {
                println!("{text}");
            } else {
                let state: TerrariumState = serde_json::from_str(&text).unwrap();
                println!("Terrarium State:");
                println!("================");
                print_terrarium_state(&state);
            }
        }
        Commands::Config {
            config_json,
            config_file,
        } => {
            log::info!("Connecting to terrarium at '{addr}'...");
            let client = reqwest::Client::new();

            let config_uri = format!("http://{addr}/config");
            if let Some(cfg) = config_json {
                let config_data: TerrariumConfigUpdate = serde_json::from_str(cfg)?;
                let resp = client.post(config_uri).json(&config_data).send().await?;
                if resp.status() != StatusCode::OK {
                    return Err(anyhow!(
                        "Got bad response: {}",
                        resp.text().await.expect("resp text")
                    ));
                }
            } else if let Some(cfg_file) = config_file {
                let file = File::open(cfg_file).expect("file read");
                let reader = BufReader::new(file);
                let config_data: TerrariumConfigUpdate = serde_json::from_reader(reader)?;
                let resp = client.post(config_uri).json(&config_data).send().await?;
                if resp.status() != StatusCode::OK {
                    return Err(anyhow!(
                        "Got bad response: {}",
                        resp.text().await.expect("resp text")
                    ));
                }
            } else {
                let resp = client.get(config_uri).send().await?;
                if resp.status() != StatusCode::OK {
                    return Err(anyhow!(
                        "Got bad response: {}",
                        resp.text().await.expect("resp text")
                    ));
                }

                let text = resp.text().await.unwrap();
                if args.json {
                    println!("{text}");
                } else {
                    // TODO: format it not as json
                    println!("{text}");
                }
            }
        }
        Commands::Scan { timeout } => {
            // Create a daemon
            let mdns = ServiceDaemon::new().expect("Failed to create daemon");

            // Browse for a service type.
            let service_type = "_oasis_terrarium._tcp.local.";
            let receiver = mdns.browse(service_type).expect("Failed to browse");

            // Keep track of entries we've seen before to avoid printing duplicates.
            let mut found = HashSet::new();

            // Receive the browse events in sync or async. Here is
            // an example of using a thread. Users can call `receiver.recv_async().await`
            // if running in async environment.
            log::info!(
                "Scanning local network (mdns) for {}s...",
                timeout.as_secs()
            );
            std::thread::spawn(move || {
                while let Ok(event) = receiver.recv() {
                    match event {
                        ServiceEvent::ServiceResolved(full_info) => {
                            let ip = full_info
                                .get_addresses_v4()
                                .iter()
                                .exactly_one()
                                .expect("one and only one ip address")
                                .to_string();

                            let info = MdnsInfo {
                                fullname: full_info.get_fullname().into(),
                                hostname: full_info
                                    .get_hostname()
                                    .strip_suffix(".")
                                    .expect("")
                                    .into(),
                                ip,
                            };
                            let is_new = found.insert(info.clone());
                            if is_new {
                                println!(
                                    "Detected terrarium with hostname '{}', ip address: '{}'",
                                    info.hostname, info.ip
                                );
                            }
                        }
                        _other_event => {
                            // println!("Received other event: {:?}", &other_event);
                        }
                    }
                }
            });

            // Gracefully shutdown the daemon after waiting a bit.
            std::thread::sleep(*timeout);
            mdns.shutdown().unwrap();
        }
    }

    Ok(())
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct MdnsInfo {
    ip: String,
    fullname: String,
    hostname: String,
}

fn create_update_data(cmds: &[ControlCommand]) -> ActuatorOverrideSet {
    let updates = cmds
        .iter()
        .map(|cmd| {
            let val = if cmd.actuator == Actuator::Mist || cmd.actuator == Actuator::Fans {
                ActuatorValue::Bool(cmd.value > 0.0)
            } else {
                ActuatorValue::Float(cmd.value)
            };
            ActuatorOverride {
                actuator: cmd.actuator,
                value: val,
                duration_secs: cmd.duration as u32,
            }
        })
        .collect();

    ActuatorOverrideSet { updates }
}

#[derive(PartialEq, Debug)]
struct ControlCommand {
    actuator: Actuator,
    value: f32,    // TODO: or bool
    duration: f32, // in seconds
}

// TODO: consider using anyhow error instead since all we're doing is recording an error message
#[derive(PartialEq, Debug)]
struct CommandParseError {
    msg: String,
}

impl CommandParseError {
    pub fn new(msg: &str) -> Self {
        Self { msg: msg.into() }
    }
}

impl fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

const DEFAULT_DURATION: f32 = 60.0;

// TODO: mist and fan should be 0 or 1, lights should be in [0, 1]
fn parse_cmd(cmd: &str) -> Result<ControlCommand, CommandParseError> {
    if cmd.is_empty() {
        return Err(CommandParseError::new("Empty command"));
    }

    let abbrev_map = HashMap::from([
        ("m", Actuator::Mist),
        ("l", Actuator::Lights),
        ("f", Actuator::Fans),
    ]);

    let re = Regex::new(
        r"^(?<abbrev>[a-zA-Z])(@(?<value>[0-9]*(\.[0-9]+)?))?(:(?<duration>[0-9]*(\.[0-9]+)?))?$",
    )
    .unwrap();
    let caps = match re.captures(cmd) {
        Some(c) => c,
        None => {
            return Err(CommandParseError::new(&format!("Invalid command: '{cmd}'")));
        }
    };

    let abbrev = caps.name("abbrev").unwrap().as_str();
    let actuator = match abbrev_map.get(abbrev.to_lowercase().as_str()) {
        Some(actuator) => *actuator,
        None => {
            return Err(CommandParseError::new(&format!(
                "Invalid abbreviation: '{abbrev}'"
            )));
        }
    };

    let value = match caps.name("value") {
        Some(mstr) => mstr.as_str().parse::<f32>().unwrap_or(0.0),
        None => {
            // If no explicit value is given, use 1 if abbrev is uppercase, else
            // 0.
            if abbrev.chars().all(|x| x.is_uppercase()) {
                1.0
            } else {
                0.0
            }
        }
    };

    let duration = match caps.name("duration") {
        Some(mstr) => mstr.as_str().parse::<f32>().unwrap_or(0.0),
        None => DEFAULT_DURATION,
    };

    Ok(ControlCommand {
        actuator,
        value,
        duration,
    })
}

#[cfg(test)]
mod parse_cmd {
    use super::*;

    #[test]
    fn full_cmd() {
        assert_eq!(
            parse_cmd("m@1:2"),
            Ok(ControlCommand {
                actuator: Actuator::Mist,
                value: 1.0,
                duration: 2.0,
            })
        );
    }

    #[test]
    fn only_duration() {
        assert_eq!(
            parse_cmd("m:2"),
            Ok(ControlCommand {
                actuator: Actuator::Mist,
                value: 0.0,
                duration: 2.0,
            })
        );
    }

    #[test]
    fn only_value() {
        assert_eq!(
            parse_cmd("m@1"),
            Ok(ControlCommand {
                actuator: Actuator::Mist,
                value: 1.0,
                duration: DEFAULT_DURATION,
            })
        );
    }

    #[test]
    fn uppercase_abbrev() {
        assert_eq!(
            parse_cmd("M:2"),
            Ok(ControlCommand {
                actuator: Actuator::Mist,
                value: 1.0,
                duration: 2.0,
            })
        );
    }

    #[test]
    fn decimals() {
        assert_eq!(
            parse_cmd("l@0.5:0.5"),
            Ok(ControlCommand {
                actuator: Actuator::Lights,
                value: 0.5,
                duration: 0.5,
            })
        );
    }

    #[test]
    fn invalid_abbrev() {
        assert_eq!(
            parse_cmd("x:2"),
            Err(CommandParseError::new("Invalid abbreviation: 'x'")),
        );
    }

    #[test]
    fn invalid_command() {
        assert_eq!(
            parse_cmd("abc"),
            Err(CommandParseError::new("Invalid command: 'abc'"))
        );
    }
}

#[cfg(test)]
mod update_data {
    use super::*;

    #[test]
    fn create() {
        let cmds = vec![
            ControlCommand {
                actuator: Actuator::Mist,
                value: 1.0,
                duration: 10.0,
            },
            ControlCommand {
                actuator: Actuator::Lights,
                value: 0.5,
                duration: 10.0,
            },
        ];
        let update_data = create_update_data(&cmds);
        assert_eq!(
            update_data,
            ActuatorOverrideSet {
                updates: vec![
                    ActuatorOverride {
                        actuator: Actuator::Mist,
                        value: ActuatorValue::Bool(true),
                        duration_secs: 10,
                    },
                    ActuatorOverride {
                        actuator: Actuator::Lights,
                        value: ActuatorValue::Float(0.5),
                        duration_secs: 10,
                    }
                ],
            }
        );

        assert_eq!(
            serde_json::to_string(&update_data).unwrap(),
            "{\"updates\":[{\"actuator\":\"mist\",\"value\":true,\"duration_secs\":10},{\"actuator\":\"lights\",\"value\":0.5,\"duration_secs\":10}]}".to_string()
        );
    }
}
