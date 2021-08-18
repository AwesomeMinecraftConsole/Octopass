pub mod proto {
    tonic::include_proto!("awesome_minecraft_console.weaver");
}

pub use proto::{Command, Line, Operation, operation, Notification};
pub use proto::weaver_server::WeaverServer;
use proto::weaver_server::*;

use futures_core::Stream;
use std::pin::Pin;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};

#[derive(Debug)]
pub struct WeaverService {
    command_sender: broadcast::Sender<String>,
    line_sender: broadcast::Sender<String>,
    operation_sender: broadcast::Sender<Operation>,
    notification_sender: broadcast::Sender<String>,
}

impl WeaverService {
    pub fn new(
        command_sender: broadcast::Sender<String>,
        line_sender: broadcast::Sender<String>,
        operation_sender: broadcast::Sender<Operation>,
        notification_sender: broadcast::Sender<String>,
    ) -> Self {
        WeaverService {
            command_sender,
            line_sender,
            operation_sender,
            notification_sender,
        }
    }
}

#[tonic::async_trait]
impl Weaver for WeaverService {
    type ConsoleStream =
        Pin<Box<dyn Stream<Item = Result<proto::Command, tonic::Status>> + Send + Sync + 'static>>;
    async fn console(
        &self,
        request: tonic::Request<tonic::Streaming<proto::Line>>,
    ) -> Result<tonic::Response<Self::ConsoleStream>, tonic::Status> {
        let line_sender = self.line_sender.clone();
        let mut stream = request.into_inner();
        tokio::spawn(async move {
            while let Ok(Some(line)) = stream.message().await {
                if let Err(_) = line_sender.send(line.line) {
                    break;
                }
            }
        });

        let (tx, rx) = mpsc::channel(32);
        let mut command_receiver = self.command_sender.subscribe();
        tokio::spawn(async move {
            while let Ok(command) = command_receiver.recv().await {
                if let Err(_) = tx.try_send(Ok(proto::Command { command })) {
                    break;
                }
            }
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    type ManagementStream =
        Pin<Box<dyn Stream<Item = Result<proto::Operation, Status>> + Send + Sync + 'static>>;

    async fn management(
        &self,
        request: tonic::Request<tonic::Streaming<proto::Notification>>,
    ) -> Result<tonic::Response<Self::ManagementStream>, tonic::Status> {
        let notification_sender = self.notification_sender.clone();
        let mut stream = request.into_inner();
        tokio::spawn(async move {
            while let Ok(Some(notification)) = stream.message().await {
                if let Err(_) = notification_sender.send(notification.notification) {
                    break;
                }
            }
        });

        let (tx, rx) = mpsc::channel(32);
        let mut operation_sender = self.operation_sender.subscribe();
        tokio::spawn(async move {
            while let Ok(operation) = operation_sender.recv().await {
                if let Err(_) = tx.try_send(Ok(operation)) {
                    break;
                }
            }
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}
