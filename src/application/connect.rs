use anyhow::{anyhow, Context, Result};
use dialoguer::{theme::ColorfulTheme, Select};
use rusoto_ec2::{filter, DescribeInstancesRequest, Ec2, Ec2Client};
use rusoto_ecs::{
    DescribeContainerInstancesRequest, DescribeTasksRequest, Ecs, EcsClient, ListTasksRequest,
};
use std::collections::HashMap;

use crate::domain;

pub struct Handler<'a> {
    ecs_client: &'a EcsClient,
    ec2_client: &'a Ec2Client,
}

impl<'a> Handler<'a> {
    pub fn new(ecs_client: &'a EcsClient, ec2_client: &'a Ec2Client) -> Self {
        Self {
            ecs_client,
            ec2_client,
        }
    }

    pub async fn connect(&self, dns_name: String, target: &str, cmd: Vec<String>) -> Result<()> {
        match target {
            target if target.contains('#') => {
                let parts: Vec<&str> = target.split('#').take(3).collect();

                if let [cluster, service] = parts[..] {
                    let mut connections = self.build_connections(cluster, service).await?;
                    self.add_containers_to_connections(cluster, &mut connections)
                        .await?;
                    self.add_ec2_instance_ids_to_connections(cluster, &mut connections)
                        .await?;
                    self.add_name_and_ip(&mut connections).await?;

                    if let Some(connection) = connections.get_connections().first() {
                        if connection.get_containers().len() > 1 {
                            let mut names: Vec<String> = connection
                                .get_containers()
                                .iter()
                                .map(|container| container.name.clone())
                                .collect();
                            names.sort();

                            return Err(anyhow!(format!(
                                        "Ambiguous connection options. Specify container with {}#{}#{{{}}}.",
                                        cluster,
                                        service,
                                        names.join(", ")
                            )));
                        }
                    }

                    if connections.get_connections().len() > 1 {
                        let options = connections.get_connection_choices();
                        let theme = ColorfulTheme::default();
                        let mut selection = Select::with_theme(&theme);
                        selection.with_prompt("Select the instance to connect to");
                        selection.items(options.as_slice());

                        let selection_choice = selection
                            .interact()
                            .context("Selection cancelled. Exiting.")?;
                        println!("{:?}", &options[selection_choice]);
                    } else {
                        let options = connections.get_connection_choices();
                        println!("{:?}", &options[0]);
                    }

                    return Ok(());
                }

                Ok(())
            }
            target if target.contains('@') => {
                let parts: Vec<&str> = target.split("@").collect();
                let choices = self.get_connection_options_for_host(parts[1]).await?;

                println!("{:?}", choices);

                Ok(())
            }
            _ => {
                let parts: Vec<&str> = target.split("@").collect();
                let choices = self.get_connection_options_for_host(target).await?;

                println!("{:?}", choices);

                Ok(())
            }
        }
    }

    async fn build_connections(
        &self,
        cluster: &str,
        service: &str,
    ) -> Result<domain::connections::Connections> {
        let mut request = ListTasksRequest::default();
        request.cluster = Some(cluster.into());
        request.service_name = Some(service.into());

        let result = self
            .ecs_client
            .list_tasks(request)
            .await
            .context("Unable to find tasks for specified cluster and service")?;

        let mut connections = domain::connections::Connections::new();

        for task_arn in result.task_arns.unwrap_or_default() {
            let task_id = task_arn.split("/").last().unwrap().to_string();
            connections.add_connection(task_id, domain::connections::Connection::new());
        }

        Ok(connections)
    }

    async fn add_containers_to_connections(
        &self,
        cluster: &str,
        connections: &mut domain::connections::Connections,
    ) -> Result<()> {
        let mut request = DescribeTasksRequest::default();
        request.cluster = Some(cluster.into());
        request.tasks = connections.task_ids();

        let result = self
            .ecs_client
            .describe_tasks(request)
            .await
            .context("Unable to describe tasks")?;

        for task in result.tasks.unwrap_or_default() {
            if task.container_instance_arn.is_none() {
                continue;
            }

            let container_instance_arn = task.container_instance_arn.unwrap();
            for container in task.containers.unwrap_or_default() {
                if container.runtime_id.is_none() || container.name.is_none() {
                    continue;
                }

                let task_id = container
                    .task_arn
                    .unwrap()
                    .split("/")
                    .last()
                    .unwrap()
                    .to_string();

                connections.add_container(
                    task_id.clone(),
                    domain::connections::Container {
                        runtime_id: container.runtime_id.unwrap(),
                        name: container.name.unwrap(),
                    },
                );

                connections.set_container_instance_id(
                    task_id.clone(),
                    container_instance_arn
                        .split("/")
                        .last()
                        .unwrap()
                        .to_string(),
                );
            }
        }

        Ok(())
    }

    async fn add_ec2_instance_ids_to_connections(
        &self,
        cluster: &str,
        connections: &mut domain::connections::Connections,
    ) -> Result<()> {
        let mut request = DescribeContainerInstancesRequest::default();
        request.cluster = Some(cluster.into());
        request.container_instances = connections.container_arns();

        let result = self
            .ecs_client
            .describe_container_instances(request)
            .await
            .context("Unable to describe container instances")?;

        for container_instance in result.container_instances.unwrap_or_default() {
            connections.set_ec2_instance_id(
                container_instance
                    .container_instance_arn
                    .unwrap()
                    .split("/")
                    .last()
                    .unwrap()
                    .to_string(),
                container_instance.ec_2_instance_id.unwrap(),
            );
        }

        Ok(())
    }

    async fn add_name_and_ip(
        &self,
        connections: &mut domain::connections::Connections,
    ) -> Result<()> {
        let mut request = DescribeInstancesRequest::default();
        request.instance_ids = Some(connections.instance_ids());

        let result = self
            .ec2_client
            .describe_instances(request)
            .await
            .context("Unable to describe instances")?;

        for reservation in result.reservations.unwrap_or_default() {
            for instance in reservation.instances.unwrap_or_default() {
                let instance_id = instance.instance_id.unwrap();
                let private_ip = instance.private_ip_address.unwrap();

                let tag_map = instance
                    .tags
                    .unwrap_or_default()
                    .iter()
                    .map(|tag| (tag.key.clone().unwrap(), tag.value.clone().unwrap()))
                    .collect::<HashMap<String, String>>();

                let name = tag_map.get("Name").unwrap().to_owned();

                connections.set_name_and_ip(instance_id, name, private_ip);
            }
        }

        Ok(())
    }

    async fn get_connection_options_for_host(&self, host: &str) -> Result<Vec<(String, String)>> {
        let mut request = DescribeInstancesRequest::default();
        request.filters = Some(vec![
            filter!("instance-state-name", "running"),
            filter!("tag:Name", host),
        ]);

        // TODO(mmk) We need to handle pagination of results
        let result = self
            .ec2_client
            .describe_instances(request)
            .await
            .context("Failed to retrieve ec2 instances")?;

        let mut choices: Vec<(String, String)> = Vec::new();

        let reservations = result.reservations.unwrap_or_default();
        for reservation in reservations {
            let instances = reservation.instances.unwrap_or_default();
            for instance in instances {
                match instance.private_ip_address {
                    Some(ip) => {
                        let instance_id =
                            instance.instance_id.unwrap_or("Unknown instance id".into());
                        choices.push((ip, instance_id));
                    }
                    _ => continue,
                };
            }
        }

        Ok(choices)
    }
}
