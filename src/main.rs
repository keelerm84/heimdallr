use anyhow::{anyhow, Result};
use rusoto_core::{region, HttpClient};
use rusoto_credential::ProfileProvider;
use rusoto_ec2::Ec2Client;
use rusoto_ecs::EcsClient;
use structopt::StructOpt;

mod application;
mod domain;
mod settings;
mod ui;

/// Connect to AWS EC2 hosts via a Bastion / Jump host
#[derive(StructOpt)]
#[structopt(name = "heimdallr", global_settings = &[structopt::clap::AppSettings::AllowLeadingHyphen])]
struct Heimdallr {
    /// Profile name as specified in your configuration file
    #[structopt(name = "profile", long, short = "p", default_value = "default")]
    profile: String,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt)]
enum Command {
    /// List all running instances
    List,

    /// Add your IP to a security group to allow ingress
    Grant {
        /// Override the security group id that controls ingress to the bastion server for the
        /// specified profile
        #[structopt(name = "security-group-id", long, short = "s")]
        security_group_id: Option<String>,

        /// Descriptive text to include with your security group entry
        #[structopt(name = "description", long, short = "d")]
        description: Option<String>,
    },

    /// Revoke your IP from a security group to prevent future ingress
    Revoke {
        /// Override the security group id that controls ingress to the bastion server for the
        /// specified profile
        #[structopt(name = "security-group-id", long, short = "s")]
        security_group_id: Option<String>,
    },

    /// Connect to a running instance
    Connect {
        /// Override the host name of the bastion server for the specified profile
        #[structopt(name = "dns-name", long, short = "d")]
        dns_name: Option<String>,

        /// Override the ssh port of the bastion server for the specified profile
        #[structopt(name = "bastion-port", long, short = "p")]
        bastion_port: Option<u16>,

        /// Override the ssh user of the bastion server for the specified profile
        #[structopt(name = "bastion-user", long, short = "u")]
        bastion_user: Option<String>,

        /// Override the user of the ec2 server for the specified profile
        #[structopt(name = "ec2-user", long, short = "e")]
        ec2_user: Option<String>,

        // TODO(mmk) Is there a better variable type to verify that the file exists?
        /// Override the ssh identity file to use for the specified profile
        #[structopt(name = "identity-file", long, short = "i")]
        identity_file: Option<String>,

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
    let settings = settings::Settings::new()?;
    let opt = Heimdallr::from_args();

    let profile_settings = match settings.profiles.get(&opt.profile) {
        Some(s) => Ok(s),
        _ => Err(anyhow!(
            "Could not find specified profile entry {}. Please check your configuration file.",
            &opt.profile
        )),
    }?;

    let mut provider = ProfileProvider::new()?;
    provider.set_profile(profile_settings.aws_profile.clone());

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
                .grant_access(
                    security_group_id.unwrap_or_else(|| profile_settings.security_group_id.clone()),
                    description,
                )
                .await
        }
        Command::Revoke { security_group_id } => {
            security_group_handler
                .revoke_access(
                    security_group_id.unwrap_or_else(|| profile_settings.security_group_id.clone()),
                )
                .await
        }
        Command::Connect {
            dns_name,
            bastion_port,
            bastion_user,
            ec2_user,
            identity_file,
            target,
            cmd,
        } => {
            ui::connect::connect(
                connect_handler,
                dns_name.unwrap_or_else(|| profile_settings.dns_name.clone()),
                bastion_port.unwrap_or_else(|| profile_settings.bastion_port),
                bastion_user.unwrap_or_else(|| profile_settings.bastion_user.clone()),
                ec2_user.unwrap_or_else(|| profile_settings.ec2_user.clone()),
                identity_file.unwrap_or_else(|| profile_settings.identity_file.clone()),
                &target,
                cmd,
            )
            .await
        }
        Command::Update => {
            tokio::task::spawn_blocking(move || {
                let status = self_update::backends::github::Update::configure()
                    .repo_owner("keelerm84")
                    .repo_name(env!("CARGO_PKG_NAME"))
                    .bin_name("heimdallr")
                    .show_download_progress(true)
                    .current_version(env!("CARGO_PKG_VERSION"))
                    .build()?
                    .update()?;
                println!("Update status: `{}`!", status.version());
                Ok(())
            })
            .await?
        }
    }
}
