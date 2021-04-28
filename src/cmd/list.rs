use anyhow::{Context, Result};
use itertools::Itertools;
use prettytable::{cell, format, row, Table};
use rusoto_core::region;
use rusoto_ec2::{filter, DescribeInstancesRequest, Ec2, Ec2Client};
use std::collections::HashMap;

pub async fn list_running_instances() -> Result<()> {
    let client = Ec2Client::new(region::Region::UsEast1);
    let mut running_instances: HashMap<String, Vec<(String, String)>> = HashMap::new();

    let mut next_token = None;

    loop {
        let mut request = DescribeInstancesRequest::default();
        request.filters = Some(vec![filter!("instance-state-name", "running")]);
        request.next_token = next_token;

        // TODO(mmk) We need to handle the next_token functionality so we can retrieve all matches
        let result = client
            .describe_instances(request)
            .await
            .context("Failed to retrieve ec2 instances")?;

        let reservations = result.reservations.unwrap_or_default();

        for reservation in reservations {
            let instances = reservation.instances.unwrap_or_default();
            for instance in instances {
                let instance_id = instance.instance_id.unwrap_or("Unknown instance id".into());

                let tag_map = instance
                    .tags
                    .unwrap_or_default()
                    .iter()
                    .map(|tag| (tag.key.clone().unwrap(), tag.value.clone().unwrap()))
                    .collect::<HashMap<String, String>>();

                let env = tag_map.get("Env").unwrap().to_owned();
                let name = tag_map.get("Name").unwrap().to_owned();

                running_instances
                    .entry(env)
                    .or_insert(Vec::new())
                    .push((name, instance_id));
            }
        }

        next_token = result.next_token;
        if next_token.is_none() {
            break;
        }
    }

    print_running_instances(running_instances);

    Ok(())
}

fn print_running_instances(instances: HashMap<String, Vec<(String, String)>>) {
    let format = format::FormatBuilder::new()
        .column_separator('│')
        .borders('│')
        .separators(
            &[format::LinePosition::Title],
            format::LineSeparator::new('─', '┼', '├', '┤'),
        )
        .padding(1, 1)
        .build();
    let mut table = Table::new();
    table.set_format(format);
    table.set_titles(row![Fgb->"Environment", Fgb->"Name", Fgb->"Instance Id"]);

    let environment_count = instances.keys().count();
    for (i, env) in instances.keys().sorted().enumerate() {
        let mut instances = instances.get(env).unwrap().to_owned();
        instances.sort_by(|lhs, rhs| lhs.0.partial_cmp(&rhs.0).unwrap());

        for instance in instances {
            table.add_row(row![Fbb->env, Fyb->instance.0, Fcb->instance.1]);
        }

        if i + 1 != environment_count {
            table.add_row(row![]);
        }
    }

    table.printstd();
}
