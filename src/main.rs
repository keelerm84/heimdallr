use anyhow::Result;
use rusoto_core::{region, HttpClient};
use rusoto_credential::ProfileProvider;
use rusoto_ec2::Ec2Client;
use rusoto_ecs::EcsClient;
use structopt::StructOpt;

mod application;
mod domain;
mod ui;

/// Connect to AWS EC2 hosts via a Bastion / Jump host
#[derive(StructOpt)]
#[structopt(name = "heimdallr", global_settings = &[structopt::clap::AppSettings::AllowLeadingHyphen])]
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
        // TODO(mmk) Security group will eventually be optional once config support is added
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
    Connect {
        /// The host name of the bastion server
        #[structopt(name = "dns-name", long, short = "d")]
        dns_name: String,

        /// The target to connect. Supported formats are host, user@host, cluster#service,
        /// cluster#service#container
        #[structopt()]
        target: String,

        /// An optional command to execute on the specified target
        #[structopt(default_value = "bash")]
        cmd: Vec<String>,
    },

    /// Update this executable to the latest version
    Update,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Heimdallr::from_args();

    let mut provider = ProfileProvider::new()?;
    provider.set_profile(opt.profile.unwrap_or("default".into()));

    let ec2_client = Ec2Client::new_with(
        HttpClient::new().unwrap(),
        provider.clone(),
        region::Region::UsEast1,
    );
    let ecs_client = EcsClient::new_with(
        HttpClient::new().unwrap(),
        provider,
        region::Region::UsEast1,
    );

    let security_group_handler = application::security_groups::Handler::new(&ec2_client);
    let list_instances_handler = application::list_instances::Handler::new(&ec2_client);
    let connect_handler = application::connect::Handler::new(&ecs_client, &ec2_client);

    match opt.cmd {
        Command::List => ui::list::list(list_instances_handler).await,
        Command::Grant {
            security_group_id,
            description,
        } => {
            security_group_handler
                .grant_access(security_group_id, description)
                .await
        }
        Command::Revoke { security_group_id } => {
            security_group_handler
                .revoke_access(security_group_id)
                .await
        }
        Command::Connect {
            dns_name,
            target,
            cmd,
        } => ui::connect::connect(connect_handler, dns_name, &target, cmd).await,
        _ => Ok(()),
    }
}
