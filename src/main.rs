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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let outward_addr = "127.0.0.1:50051".parse().unwrap();
    let inward_addr = "127.0.0.1:50052".parse().unwrap();

    let (command_sender, mut command_receiver) = broadcast::channel(32);
    let (line_sender, mut line_receiver) = broadcast::channel(32);
    let (operation_sender, mut operation_receiver) = broadcast::channel(32);
    let (notification_sender, mut notification_receiver) = broadcast::channel(32);
    let (online_players_sender, mut online_players_receiver) = broadcast::channel(32);

    tokio::spawn(async move {
        loop {
            let line = line_receiver.recv().await;
            println!("{}", line.unwrap());
        }
    });

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
        inward = Server::builder().add_service(weaver_server).add_service(acrobat_server).serve(inward_addr) => {
            println!("inward service has finished");
            if let Err(error) = inward {
                println!("{}", error);
            }
        }
        outward = Server::builder().add_service(endervision_server).serve(outward_addr) => {
            println!("outward service has finished");
            if let Err(error) = outward {
                println!("{}", error);
            }
        }
    }

    Ok(())
}
