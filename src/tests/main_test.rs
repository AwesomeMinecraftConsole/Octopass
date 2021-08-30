#![feature(async_stream)]
#![feature(in_band_lifetimes)]

use tokio::sync::broadcast;
use tokio::sync::mpsc;

use tonic::{transport::Server, Status};

mod endervision;
mod weaver;
mod acrobat;

use weaver::{WeaverService, WeaverServer};
use acrobat::{AcrobatService, AcrobatServer};
use endervision::{EnderVisionService, EnderVisionServer};

#[tokio::test]
#[ignore]
async fn main_test() -> Result<(), Box<dyn std::error::Error>> {
    let endervision_host = "127.0.0.1:50051".parse()?
    let waver_and_acrobat_host = "127.0.0.1:50052".parse()?;

    let (command_sender, mut command_receiver) = broadcast::channel(32);
    let (line_sender, mut line_receiver) = broadcast::channel(32);
    let (operation_sender, mut operation_receiver) = broadcast::channel(32);
    let (notification_sender, mut notification_receiver) = broadcast::channel(32);
    let (online_players_sender, mut online_players_receiver) = broadcast::channel(32);

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

    tokio::select! {
        acrobat = Server::builder().add_service(weaver_server).add_service(acrobat_server).serve(waver_and_acrobat_host) => {}
        endervision = Server::builder().add_service(endervision_server).serve(endervision_host) => {}
        _ = async move {
            loop {
                let line = line_receiver.recv().await;
                println!("{}", line.unwrap());
            }
        }
        _ = async move {
            loop {
                let command = command_receiver.recv().await;
                println!("{}", command.unwrap());
            }
        }
    }

    Ok(())
}