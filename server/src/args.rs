use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "agentsecret-server",
    version,
    about = "AgentSecret HTTP server"
)]
pub struct Args {
    #[arg(long, env = "DATABASE_URL")]
    pub database_url: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 仅在空库中创建初始管理员
    SeedAdmin {
        #[arg(long)]
        email: String,

        #[arg(long)]
        name: String,
    },
}
