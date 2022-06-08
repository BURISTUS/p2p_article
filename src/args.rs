use clap::{
    Args,
    Parser,
    Subcommand,
};

use libp2p::PeerId;

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct AppArgs{
    #[clap(subcommand)]
    pub command_type: CommandType,
}

#[derive(Debug, Subcommand)]
pub enum CommandType {
    ///user crud
    CreateUser(CreateUser),
    //article cruview users or articles
    ListShowPeers(ListShowPeers),

    ListShowArticle(ListShowAricles),
//    ListShowAllArticles(ListShowAllArticles);
    CreateArticle(CreateArticle)
}

#[derive(Debug, Args)]
pub struct CreateUser{

    /// User name
    pub name: String,
    /// User email
    pub email: String,

}

#[derive(Debug, Args)]
pub struct UpdateUser{
    /// Update user name
    pub new_name: String,
    /// Update user email
    pub new_email: String
}

#[derive(Debug, Args)]
pub struct ListShowPeers;

#[derive(Debug, Args)]
pub struct ListShowPeer{
    id: PeerId,
}

#[derive(Debug, Args)]
pub struct ListShowAricles{
   pub id: usize,
}

#[derive(Debug, Args)]
pub struct CreateArticle{
    pub name: String,
    pub text: String,
}
