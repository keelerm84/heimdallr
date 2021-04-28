use anyhow::Result;
use structopt::StructOpt;

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
        /// Descriptive text to include with your security group entry
        #[structopt(name = "description", long, short = "d")]
        description: Option<String>,
    },

    /// Revoke your IP from a security group to prevent future ingress
    Revoke,

    /// Connect to a running instance
    Connect,

    /// Update this executable to the latest version
    Update,
}

fn main() -> Result<()> {
    let opt = Heimdallr::from_args();
    Ok(())
}
