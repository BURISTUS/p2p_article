use log::{info, error};
use libp2p::swarm::Swarm;
use tokio::{fs, sync::mpsc};
use libp2p::floodsub;
use crate::{Articles, ARTICLES_STORAGE_FILE_PATH, Article, ArticleBehaviour, ListRequest, TOPIC, ListResponse, ListMode};
use std::collections::HashSet;


type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

pub async fn handle_peer_list(swarm: &mut Swarm<ArticleBehaviour>){
    info!("Discovered peers:");
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes{
        unique_peers.insert(peer);
    }
    unique_peers.iter().for_each(|e| info!("{}", e));
}

pub async fn handle_create_article(name: &str, text: &str){
   if let Err(e) = create_new_article(name, text).await {
       error!("Error when creating the article {}", e);
   }
}

pub async fn handle_list_article(id: usize, swarm: &mut Swarm<ArticleBehaviour>){
    let req = ListRequest {
        mode: crate::ListMode::One(id.to_string()),
    };
    let json = serde_json::to_string(&req).expect("can't pack to json");
    swarm.behaviour_mut().floodsub.publish(TOPIC.clone(), json.as_bytes());
}

async fn read_local_articles() -> Result<Articles>{
    let content = fs::read(ARTICLES_STORAGE_FILE_PATH).await?;
    let res = serde_json::from_slice(&content)?;
    Ok(res)
}

async fn write_local_articles(articles: &Articles) -> Result<()> {
    let json = serde_json::to_string(&articles)?;
    fs::write(ARTICLES_STORAGE_FILE_PATH, &json).await?;
    Ok(())
}

async fn create_new_article(name: &str, text: &str) -> Result<()>{
    let mut local_articles = read_local_articles().await?;
    let new_id = match local_articles.iter().max_by_key(|r| r.id) {
        Some(i) => i.id + 1,
        None => 0
    };

    local_articles.push(Article{
        id: new_id,
        name: name.to_owned(),
        description: text.to_owned(),
        public: false
    });

    write_local_articles(&local_articles).await?;

    Ok(())
}

pub async fn respond_with_public_articles(sender: mpsc::UnboundedSender<ListResponse>, receiver: String){
    tokio::spawn(async move {
        match read_local_articles().await {
            Ok(recipes) => {
                let resp = ListResponse {
                    mode: ListMode::ALL,
                    receiver,
                    data: recipes.into_iter().filter(|r| r.public).collect(),
                };
                if let Err(e) = sender.send(resp) {
                    error!("error sending response via channel, {}", e);
                }
            }
            Err(e) => error!("error fetching local recipes to answer ALL request, {}", e),
        }
    });
}
