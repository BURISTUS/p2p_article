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
use std::{collections::HashSet, convert::identity};
use tokio::{fs, io::AsyncBufReadExt, sync::mpsc};

const STORAGE_FILE_PATH: &str = "./recipes.json";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("recipes"));

type Recipes = Vec<Recipe>;

#[derive(Debug, Serialize, Deserialize)]
struct Recipe{
    id: usize,
    name: String,
    ingredients: String,
    instruction: String,
    public: bool
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
struct ListResponce{
    mode: ListMode,
    data: Recipes,
    receiver: String
}

enum EventType{
    Response(ListResponce),
    Input(String)
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

    let mut behavior = RecipeBehaviour {
        //TODO
    };


    let mut swarm = SwarmBuilder::new(transp, behavior, PEER_ID.clone())
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

    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    loop {
        let evt = {
            tokio::select! {
                line = stdin.next_line() => Some(EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
                event = swarm.next() => {
                    info!("Unhandled Swarm Event: {:?}", event);
                    None
                },
                response = response_recv.recv() => Some(EventType::Response(response.expect("response exists"))),
            }
        };
    }

}
