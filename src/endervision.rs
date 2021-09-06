use std::pin::Pin;

use futures_core::Stream;
use log::{info, warn};
use tokio::sync::{broadcast, TryAcquireError};
use tokio::sync::broadcast::error::*;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Code, Response, Status};

use proto::*;
use proto::ender_vision_server::EnderVision;
pub use proto::ender_vision_server::EnderVisionServer;

use crate::acrobat;
use crate::weaver;

mod proto {
    tonic::include_proto!("awesome_minecraft_console.endervision");
    tonic::include_proto!("google.protobuf");
}

#[derive(Debug)]
pub struct EnderVisionService {
    command_sender: broadcast::Sender<String>,
    line_sender: broadcast::Sender<String>,
    operation_sender: broadcast::Sender<weaver::Operation>,
    notification_sender: broadcast::Sender<String>,
    online_players_sender: broadcast::Sender<acrobat::OnlinePlayers>,
}

impl EnderVisionService {
    pub fn new(
        command_sender: broadcast::Sender<String>,
        line_sender: broadcast::Sender<String>,
        operation_sender: broadcast::Sender<weaver::Operation>,
        notification_sender: broadcast::Sender<String>,
        online_players_sender: broadcast::Sender<acrobat::OnlinePlayers>,
    ) -> Self {
        EnderVisionService {
            command_sender,
            line_sender,
            operation_sender,
            notification_sender,
            online_players_sender,
        }
    }
}

#[tonic::async_trait]
impl EnderVision for EnderVisionService {
    type ConsoleStream =
    Pin<Box<dyn Stream<Item=Result<weaver::Line, tonic::Status>> + Send + Sync + 'static>>;

    async fn console(
        &self,
        request: tonic::Request<tonic::Streaming<weaver::Command>>,
    ) -> Result<tonic::Response<Self::ConsoleStream>, tonic::Status> {
        let command_sender = self.command_sender.clone();
        let mut stream = request.into_inner();
        tokio::spawn(async move {
            while let Ok(Some(command)) = stream.message().await {
                if let Err(_) = command_sender.send(command.command) {
                    break;
                }
            }
        });

        let mut line_receiver = self.line_sender.subscribe();
        let (tx, rx) = mpsc::channel(128);
        tokio::spawn(async move {
            loop {
                match line_receiver.recv().await {
                    Ok(line) => {
                        if tx.send(Ok(weaver::Line { line })).await.is_err() {
                            warn!("endervision: error");
                            break;
                       }
                    }
                    Err(RecvError::Lagged(skipped)) => {
                        warn!("endervision: skipped {} message(s)", skipped);
                    }
                    Err(RecvError::Closed) => {
                        info!("endervision: closed connection");
                        let _ = tx.closed();
                        break;
                    }
                }
            }
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    type ManagementStream = Pin<
        Box<dyn Stream<Item=Result<weaver::Notification, Status>> + Send + Sync + 'static>,
    >;

    async fn management(
        &self,
        request: tonic::Request<tonic::Streaming<weaver::Operation>>,
    ) -> Result<tonic::Response<Self::ManagementStream>, tonic::Status> {
        let mut stream = request.into_inner();
        let mut operation_sender = self.operation_sender.clone();
        tokio::spawn(async move {
            while let Ok(Some(operation)) = stream.message().await {
                if let Err(_) = operation_sender.send(operation) {
                    break
                }
            }
        });

        let (tx, rx) = mpsc::channel(128);
        let mut notification_receiver = self.notification_sender.subscribe();
        tokio::spawn(async move {
            while let Ok(notification) = notification_receiver.recv().await {
                 if let Err(_) = tx.try_send(Ok(weaver::Notification { notification })) {
                     break;
                }
            }
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    type OnlinePlayersStream =
    Pin<Box<
        dyn futures_core::Stream<Item=Result<OnlinePlayersResponse, tonic::Status>>
        + Send
        + Sync
        + 'static,
    >>;
    async fn online_players(
        &self,
        request: tonic::Request<()>,
    ) -> Result<tonic::Response<Self::OnlinePlayersStream>, tonic::Status> {
        let (tx, rx) = mpsc::channel(128);
        let mut online_players_receiver = self.online_players_sender.subscribe();
        tokio::spawn(async move {
            while let Ok(online_players) = online_players_receiver.recv().await {
                if let Err(_) = tx.try_send(Ok(proto::OnlinePlayersResponse { online_players })) {
                    break;
                }
            }
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}
