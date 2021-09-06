#![feature(async_stream)]
#![feature(in_band_lifetimes)]

use std::env;

use log::info;
use tokio::sync::broadcast::{self, error::RecvError};

use tonic::{transport::Server, Status};

use loyalwolf::weaver::{WeaverService, WeaverServer};
use loyalwolf::acrobat::{AcrobatService, AcrobatServer};
use loyalwolf::endervision::{EnderVisionService, EnderVisionServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env::set_var("RUST_LOG", "info");
    let _ = env_logger::init();

    info!("Starting Loyalwolf...");

    let endervision_host = "127.0.0.1:50051".parse()?;
    let waver_and_acrobat_host = "127.0.0.1:50052".parse()?;

    let (command_sender, mut command_receiver) = broadcast::channel(128);
    let (line_sender, mut line_receiver) = broadcast::channel(128);
    let (operation_sender, mut operation_receiver) = broadcast::channel(128);
    let (notification_sender, mut notification_receiver) = broadcast::channel(128);
    let (online_players_sender, mut online_players_receiver) = broadcast::channel(128);

    let weaver = WeaverService::new(
        command_sender.clone(),
        line_sender.clone(),
        operation_sender.clone(),
        notification_sender.clone(),
    );
    let weaver_server = WeaverServer::new(weaver);

    let acrobat = AcrobatService::new(
        online_players_sender.clone(),
    );
    let acrobat_server = AcrobatServer::new(acrobat);

    let endervision = EnderVisionService::new(
        command_sender.clone(),
        line_sender.clone(),
        operation_sender.clone(),
        notification_sender.clone(),
        online_players_sender.clone()
    );
    let endervision_server = EnderVisionServer::new(endervision);

    tokio::spawn(async move {
        loop {
            if let Err(RecvError::Lagged(hoge)) = line_receiver.recv().await {
                println!("{}", hoge);
            }
        }
    });

    info!("Start the server now!");

    tokio::select! {
        weaver_and_acrobat = Server::builder().add_service(weaver_server).add_service(acrobat_server).serve(waver_and_acrobat_host) => {}
        endervision = Server::builder().add_service(endervision_server).serve(endervision_host) => {}
    }

    info!("The server is being terminated.");

    Ok(())
}