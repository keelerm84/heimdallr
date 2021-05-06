use crate::application::connect::Handler;
use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Select};

pub async fn connect<'a>(
    handler: Handler<'a>,
    dns_name: String,
    target: &str,
    cmd: Vec<String>,
) -> Result<()> {
    let choices = match target {
        target if target.contains('#') => {
            handler.ssh_connection_choices_for_service(target).await?
        }
        target if target.contains('@') => {
            let parts: Vec<&str> = target.split("@").collect();
            handler.ssh_connection_choices_for_host(parts[1]).await?
        }
        _ => handler.ssh_connection_choices_for_host(target).await?,
    };

    if choices.len() > 1 {
        let theme = ColorfulTheme::default();
        let mut selection = Select::with_theme(&theme);
        selection.with_prompt("Select the instance to connect to");
        selection.items(choices.as_slice());

        let selection_choice = selection
            .interact()
            .context("Selection cancelled. Exiting.")?;
        println!("{}", &choices[selection_choice]);
    } else if choices.len() == 1 {
        println!("{}", &choices[0]);
    } else {
        println!("No choice match");
    }

    Ok(())
}
