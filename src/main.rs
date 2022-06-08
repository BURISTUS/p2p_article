use libp2p::{
    core::upgrade,
    floodsub::{Floodsub, FloodsubEvent, Topic},
    futures::StreamExt,
    identity,
    mdns::{Mdns, MdnsEvent},
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{NetworkBehaviourEventProcess, Swarm, SwarmBuilder},
    tcp::TokioTcpConfig,
    NetworkBehaviour, PeerId, Transport,
};
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::{io::AsyncBufReadExt, sync::mpsc};
use clap::{Parser, Command, command};

use crate::args::{AppArgs, CreateUser, CommandType, CreateArticle, ListShowAricles, ListShowPeers};
use crate::handle::{handle_create_article, handle_list_article, respond_with_public_articles, handle_peer_list};
pub const ARTICLES_STORAGE_FILE_PATH: &str = "./articles.json";
pub const USER_STORAGE_FILE_PATH: &str = "./users.json";

mod args;
mod handle;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("articles"));

type Articles = Vec<Article>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Article{
   pub id: usize,
   pub name: String,
   pub description: String,
   pub public: bool
}

#[derive(Debug, Serialize, Deserialize)]
enum ListMode{
    ALL,
    One(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct ListRequest{
    mode: ListMode
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListResponse{
    mode: ListMode,
    data: Articles,
    receiver: String
}

enum EventType{
    Response(ListResponse),
    Input(String)
}

#[derive(NetworkBehaviour)]
pub struct ArticleBehaviour{
   pub floodsub: Floodsub,
   pub mdns: Mdns,
    #[behaviour(ignore)]
   pub response_sender: mpsc::UnboundedSender<ListResponse>
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for ArticleBehaviour{
    fn inject_event(&mut self, event: FloodsubEvent){
        match event {
            FloodsubEvent::Message(msg) => {
                if let Ok(resp) = serde_json::from_slice::<ListResponse>(&msg.data) {
                    if resp.receiver == PEER_ID.to_string() {
                        info!("Response from {}:", msg.source);
                        resp.data.iter().for_each(|r| info!("{:?}", r));
                    }
                } else if let Ok(req) = serde_json::from_slice::<ListRequest>(&msg.data) {
                    match req.mode {
                        ListMode::ALL => {
                            info!("Received ALL req: {:?} from {:?}", req, msg.source);
                            respond_with_public_articles(
                                self.response_sender.clone(),
                                msg.source.to_string(),
                            );
                        }
                        ListMode::One(ref peer_id) => {
                            if peer_id == &PEER_ID.to_string() {
                                info!("Received req: {:?} from {:?}", req, msg.source);
                                respond_with_public_articles(
                                    self.response_sender.clone(),
                                    msg.source.to_string(),
                                );
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }
}


impl NetworkBehaviourEventProcess<MdnsEvent> for ArticleBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    info!("Peer ID: {}", PEER_ID.clone());
    let (response_sender, mut response_recv) = mpsc::unbounded_channel();

    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&KEYS)
        .expect("can't create auth keys");

    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    let mut behaviour = ArticleBehaviour {
        floodsub: Floodsub::new(PEER_ID.clone()),
        mdns: Mdns::new(Default::default())
            .await
            .expect("can create mdns"),
        response_sender,
    };

    behaviour.floodsub.subscribe(TOPIC.clone());


    let mut swarm = SwarmBuilder::new(transp, behaviour, PEER_ID.clone())
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();

    Swarm::listen_on(
        &mut swarm,
        "/ip4/0.0.0.0/tcp/0"
            .parse()
            .expect("can get a local socket"),
    )
        .expect("swarm can be started");

    let args = AppArgs::parse();

    match &args.command_type {
        CommandType::CreateUser(CreateUser { name, email }) => {
            println!("test {:?} {:?}", name, email);

        },
        CommandType::CreateArticle(CreateArticle{ name, text}) => {
            println!("{:?}, {:?}", name, text);
            handle_create_article(name, text).await
        },
        CommandType::ListShowArticle(ListShowAricles{id}) => {
            println!("id is {:?}", id);
            handle_list_article(id.clone(), &mut swarm).await
        },
        CommandType::ListShowPeers(ListShowPeers) => {
            println!("show peers");
            handle_peer_list(&mut swarm).await
        }
    }
}
