mod proto {
    tonic::include_proto!("awesome_minecraft_console.acrobat");
}

pub use proto::OnlinePlayer;
pub use proto::acrobat_server::AcrobatServer;
use proto::acrobat_server::Acrobat;
use tokio::sync::broadcast;
use tonic::Response;

pub type OnlinePlayers = Vec<OnlinePlayer>;

pub struct AcrobatService {
    online_players_sender: broadcast::Sender<OnlinePlayers>
}

impl AcrobatService {
    pub fn new(online_players_sender: broadcast::Sender<OnlinePlayers>) -> Self {
        AcrobatService {
            online_players_sender
        }
    }
}

#[tonic::async_trait]
impl Acrobat for AcrobatService {
    async fn online_players(
        &self,
        request: tonic::Request<tonic::Streaming<proto::OnlinePlayersRequest>>,
    ) -> Result<tonic::Response<()>, tonic::Status> {

        let online_players_sender = self.online_players_sender.clone();
        let mut stream = request.into_inner();
        tokio::spawn(async move {
            while let Ok(Some(online_players_request)) = stream.message().await {
                if let Err(_) = online_players_sender.send(online_players_request.online_players) {
                    break;
                }
            }
        });

        Ok(Response::new(()))
    }
}