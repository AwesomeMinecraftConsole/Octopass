#![feature(async_stream)]
#![feature(in_band_lifetimes)]

use tokio::sync::broadcast;

use tonic::{transport::Server};

use loyalwolf::weaver::{WeaverService, WeaverServer};
use loyalwolf::acrobat::{AcrobatService, AcrobatServer};
use loyalwolf::endervision::{EnderVisionService, EnderVisionServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endervision_host_row = std::env::var("LOYALWOLF_ENDERVISION_HOST")? + ":" + &std::env::var("LOYALWOLF_ENDERVISION_PORT")?;
    let weaver_and_acrobat_host_row = std::env::var("LOYALWOLF_WEAVER_AND_ACROBAT_HOST")? + ":" + &std::env::var("LOYALWOLF_WEAVER_AND_ACROBAT_PORT")?;
    let endervision_host = endervision_host_row.parse()?;
    let waver_and_acrobat_host = weaver_and_acrobat_host_row.parse()?;

    let (command_sender, _) = broadcast::channel(32);
    let (line_sender, _) = broadcast::channel(32);
    let (operation_sender, _) = broadcast::channel(32);
    let (notification_sender, _) = broadcast::channel(32);
    let (online_players_sender, _) = broadcast::channel(32);

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
        _ = Server::builder().add_service(weaver_server).add_service(acrobat_server).serve(waver_and_acrobat_host) => {}
        _ = Server::builder().add_service(endervision_server).serve(endervision_host) => {}
    }

    Ok(())
}