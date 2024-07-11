use std::env;
use anyhow::Context;

const SERVER_PORT_KEY: &str = "SERVER_PORT";

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Config {
  pub server_port: String,
}

impl Config {
  pub fn from_env() -> anyhow::Result<Config> {
    let server_port = load_env(SERVER_PORT_KEY)?;

    Ok(Config {
        server_port,
        })
    }
  }
 
fn load_env(key: &str) -> anyhow::Result<String> {
  env::var(key).with_context(|| format!("failed to load environment variable {}", key))
  }
