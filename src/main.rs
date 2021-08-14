use futures_core::Stream;
use octopass::{
    console_server::{Console, ConsoleServer},
    CommandResponse, LineRequest, NotificationRequest, OperationResponse,
};
use std::pin::Pin;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status, Streaming};

mod octopass {
    tonic::include_proto!("com.uramnoil.awesome_minecraft_console.protocols");
    tonic::include_proto!("google.protobuf");
}

#[derive(Debug)]
struct ConsoleServerService {
    command_sender: broadcast::Sender<String>,
    line_sender: mpsc::Sender<Result<String, Status>>,
    operation_sender: broadcast::Sender<Operation>,
    notification_sender: mpsc::Sender<Result<String, Status>>,
}

#[derive(Clone)]
enum Operation {
    start,
}

#[tonic::async_trait]
impl Console for ConsoleServerService {
    type ConsoleStream =
        Pin<Box<dyn Stream<Item = Result<CommandResponse, tonic::Status>> + Send + Sync + 'static>>;
    async fn console(
        &self,
        request: tonic::Request<tonic::Streaming<LineRequest>>,
    ) -> Result<tonic::Response<Self::ConsoleStream>, tonic::Status> {
        let line_sender = self.line_sender.clone();
        let mut stream = request.into_inner();
        tokio::spawn(async move {
            loop {
                let result = stream.message().await;
                match result {
                    Ok(Some(..)) | Err(..) => {
                        let result = line_sender
                            .send(result.map(|option| option.unwrap().line))
                            .await;
                        if let Err(..) = result {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        });

        let (tx, rx) = mpsc::channel(32);
        let mut command_sender = self.command_sender.subscribe();
        tokio::spawn(async move {
            loop {
                let result = command_sender.recv().await;
                let result = tx
                    .send(
                        result
                            .map(|op| CommandResponse { command: op })
                            .map_err(|_| Status::internal("internal error")),
                    )
                    .await;
                if let Err(..) = result {
                    break;
                }
            }
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    type ManagementStream =
        Pin<Box<dyn Stream<Item = Result<OperationResponse, Status>> + Send + Sync + 'static>>;

    async fn management(
        &self,
        request: tonic::Request<tonic::Streaming<NotificationRequest>>,
    ) -> Result<tonic::Response<Self::ManagementStream>, tonic::Status> {
        let notification_sender = self.notification_sender.clone();
        let mut stream = request.into_inner();
        tokio::spawn(async move {
            loop {
                let result = stream.message().await;
                match result {
                    Ok(Some(..)) | Err(..) => {
                        let result = notification_sender
                            .send(result.map(|option| option.unwrap().message))
                            .await;
                        if let Err(..) = result {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        });

        let (tx, rx) = mpsc::channel(32);
        let mut operation_sender = self.operation_sender.subscribe();
        let handler = tokio::spawn(async move {
            loop {
                let result = operation_sender.recv().await;
                let result = tx
                    .send(
                        result
                            .map(|op| OperationResponse {
                                operation: match op {
                                    start => 1,
                                },
                            })
                            .map_err(|_| Status::internal("internal error")),
                    )
                    .await;
                if let Err(..) = result {
                    break;
                }
            }
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:50052".parse().unwrap();

    println!("listening on: {}", addr);

    let (command_sender, mut command_receiver) = broadcast::channel(32);
    let (line_sender, mut line_receiver) = mpsc::channel(32);
    let (operation_sender, mut operation_receiver) = broadcast::channel(32);
    let (notification_sender, mut notification_receiver) = mpsc::channel(32);

    tokio::spawn(async move {
        loop {
            let line: Result<String, Status> = line_receiver.recv().await.unwrap();
            println!("{}", line.unwrap())
        }
    });

    let console = ConsoleServerService {
        command_sender,
        line_sender,
        operation_sender,
        notification_sender,
    };

    let svc = ConsoleServer::new(console);

    Server::builder().add_service(svc).serve(addr).await?;

    Ok(())
}
