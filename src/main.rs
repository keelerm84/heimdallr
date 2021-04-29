use anyhow::Result;
use structopt::StructOpt;

mod cmd;

/// Connect to AWS EC2 hosts via a Bastion / Jump host
#[derive(StructOpt)]
#[structopt(name = "heimdallr")]
struct Heimdallr {
    /// Profile name as specified in your configuration file
    #[structopt(name = "profile", long, short = "p")]
    profile: Option<String>,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt)]
enum Command {
    /// List all running instances
    List,

    /// Add your IP to a security group to allow ingress
    Grant {
        /// The security group id that controls ingress to the bastion server
        #[structopt(name = "security-group-id", long, short = "s")]
        security_group_id: String,

        /// Descriptive text to include with your security group entry
        #[structopt(name = "description", long, short = "d")]
        description: Option<String>,
    },

    /// Revoke your IP from a security group to prevent future ingress
    Revoke {
        /// The security group id that controls ingress to the bastion server
        #[structopt(name = "security-group-id", long, short = "s")]
        security_group_id: String,
    },

    /// Connect to a running instance
    Connect,

    /// Update this executable to the latest version
    Update,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Heimdallr::from_args();

    match opt.cmd {
        Command::List => cmd::list::list_running_instances().await,
        Command::Grant {
            security_group_id,
            description,
        } => cmd::access::grant(security_group_id, description).await,
        Command::Revoke { security_group_id } => cmd::access::revoke(security_group_id).await,
        _ => Ok(()),
    }
}
