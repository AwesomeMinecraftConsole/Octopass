use futures_core::Stream;
use octopass::console_server;
use octopass::{CommandResponse, Empty, LineRequest};
use std::ffi::IntoStringError;
use std::pin::Pin;
use std::sync::mpsc::RecvError;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status, Streaming};

pub mod octopass {
    tonic::include_proto!("com.uramnoil.awesome_minecraft_console.protocols");
}

#[derive(Debug)]
pub struct ConsoleServerService {
    command_sender: broadcast::Sender<String>,
    line_sender: broadcast::Sender<Result<String, Status>>,
}

#[tonic::async_trait]
impl console_server::Console for ConsoleServerService {
    type CommandStream =
        Pin<Box<dyn Stream<Item = Result<CommandResponse, Status>> + Send + Sync + 'static>>;

    async fn command(
        &self,
        request: tonic::Request<octopass::Empty>,
    ) -> Result<Response<Self::CommandStream>, tonic::Status> {
        let (tx, mut rx) = mpsc::channel(32);
        let mut command_sender = self.command_sender.subscribe();
        tokio::spawn(async move {
            loop {
                let result = command_sender.recv().await;
                let result = tx
                    .send(
                        result
                            .map(|op| CommandResponse { message: op })
                            .map_err(|op| Status::internal("internal error")),
                    )
                    .await;
                if let Err(err) = result {
                    break;
                }
            }
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn lines(
        &self,
        request: Request<Streaming<LineRequest>>,
    ) -> Result<Response<octopass::Empty>, tonic::Status> {
        let line_sender = self.line_sender.clone();
        let mut stream = request.into_inner();
        loop {
            let result = stream.message().await;
            match result {
                Ok(Some(..)) | Err(..) => {
                    let result = line_sender.send(result.map(|option| option.unwrap().message));
                    if let Err(..) = result {
                        break;
                    }
                }
                _ => {}
            }
        }

        Ok(Response::new(octopass::Empty {}))
    }
}

fn main() {
    println!("Hello, world!");
}
