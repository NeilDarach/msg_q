use msg_q::config::Config;
use msg_q::domain::messages::service::Service;
use msg_q::inbound::http::{HttpServer,HttpServerConfig};
use msg_q::outbound::memory::Memory;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let config = Config::from_env()?;
  
  tracing_subscriber::fmt::init();

  let repo = Memory::new().await?;
  let service = Service::new(repo);
  
  let server_config = HttpServerConfig {
                       port: &config.server_port,
                       };

  let http_server = HttpServer::new(service,server_config).await?;
  http_server.run().await
  }
