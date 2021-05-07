use anyhow::{Context, Result};
use rusoto_ec2::{filter, DescribeInstancesRequest, Ec2, Ec2Client};
use std::collections::HashMap;

pub struct Handler<'a> {
    client: &'a Ec2Client,
}

impl<'a> Handler<'a> {
    pub fn new(client: &'a Ec2Client) -> Self {
        Self { client }
    }

    pub async fn list(self) -> Result<HashMap<String, Vec<(String, String)>>> {
        let mut running_instances: HashMap<String, Vec<(String, String)>> = HashMap::new();

        let mut next_token = None;

        loop {
            let mut request = DescribeInstancesRequest::default();
            request.filters = Some(vec![filter!("instance-state-name", "running")]);
            request.next_token = next_token;

            // TODO(mmk) We need to handle the next_token functionality so we can retrieve all matches
            let result = self
                .client
                .describe_instances(request)
                .await
                .context("Failed to retrieve ec2 instances")?;

            let reservations = result.reservations.unwrap_or_default();

            for reservation in reservations {
                let instances = reservation.instances.unwrap_or_default();
                for instance in instances {
                    let instance_id = instance
                        .instance_id
                        .unwrap_or_else(|| "Unknown instance id".into());

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
                        .or_insert_with(Vec::new)
                        .push((name, instance_id));
                }
            }

            next_token = result.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(running_instances)
    }
}
