use anyhow::{anyhow, Context, Result};
use rusoto_ec2::{filter, DescribeInstancesRequest, Ec2, Ec2Client};
use rusoto_ecs::{
    DescribeContainerInstancesRequest, DescribeTasksRequest, Ecs, EcsClient, ListTasksRequest,
};
use std::collections::HashMap;

// TODO(mmk) This is a smell. We probably shouldn't have to expose all of these.
use crate::domain::connections::{
    Connection, Connections, Container, HostConnection, SshConnection,
};

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

    pub async fn ssh_connection_choices_for_service(
        &self,
        target: &str,
    ) -> Result<Vec<Box<dyn SshConnection>>> {
        let parts: Vec<&str> = target.split('#').take(3).collect();

        match parts[..] {
            [cluster, service] => {
                let conns = self
                    .build_connections_for_service(cluster, service, None)
                    .await?;

                if let Some(connection) = conns.get_connections().first() {
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

                Ok(conns.get_connection_choices())
            }
            [cluster, service, container] => {
                let conns = self
                    .build_connections_for_service(cluster, service, Some(container))
                    .await?;

                Ok(conns.get_connection_choices())
            }
            _ => Err(anyhow!("Invalid target format specified.")),
        }
    }

    pub async fn ssh_connection_choices_for_host(
        &self,
        host: &str,
    ) -> Result<Vec<Box<dyn SshConnection>>> {
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

        let mut choices: Vec<Box<dyn SshConnection>> = Vec::new();

        let reservations = result.reservations.unwrap_or_default();
        for reservation in reservations {
            let instances = reservation.instances.unwrap_or_default();
            for instance in instances {
                match instance.private_ip_address {
                    Some(ip) => {
                        let instance_id = instance
                            .instance_id
                            .unwrap_or_else(|| "Unknown instance id".into());
                        choices.push(Box::new(HostConnection {
                            name: host.into(),
                            instance_id,
                            private_ip: ip,
                        }) as Box<dyn SshConnection>);
                    }
                    _ => continue,
                };
            }
        }

        Ok(choices)
    }

    async fn build_connections(&self, cluster: &str, service: &str) -> Result<Connections> {
        let mut request = ListTasksRequest::default();
        request.cluster = Some(cluster.into());
        request.service_name = Some(service.into());

        let result = self
            .ecs_client
            .list_tasks(request)
            .await
            .context("Unable to find tasks for specified cluster and service")?;

        let mut connections = Connections::new();

        for task_arn in result.task_arns.unwrap_or_default() {
            connections.add_connection(arn_to_id(&task_arn).to_string(), Connection::new());
        }

        Ok(connections)
    }

    async fn add_containers_to_connections(
        &self,
        cluster: &str,
        container_name: Option<&str>,
        connections: &mut Connections,
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

                let name = container.name.unwrap();

                if let Some(cn) = container_name {
                    if cn != name {
                        continue;
                    }
                }

                if let Some(task_arn) = container.task_arn {
                    let task_id = arn_to_id(&task_arn).to_string();

                    connections.add_container(
                        task_id.clone(),
                        Container {
                            runtime_id: container.runtime_id.unwrap(),
                            name,
                        },
                    );

                    connections.set_container_instance_id(
                        task_id,
                        arn_to_id(&container_instance_arn).to_string(),
                    );
                }
            }
        }

        Ok(())
    }

    async fn add_ec2_instance_ids_to_connections(
        &self,
        cluster: &str,
        connections: &mut Connections,
    ) -> Result<()> {
        // TODO(mmk) This return type could be more specific. We could make a custom type and use
        // it to prevent calling the other decorate methods instead of having different checks at
        // the top of each one
        if connections.container_arns().is_empty() {
            return Ok(());
        }
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
                arn_to_id(container_instance.container_instance_arn.unwrap().as_str()).to_string(),
                container_instance.ec_2_instance_id.unwrap(),
            );
        }

        Ok(())
    }

    async fn add_name_and_ip(&self, connections: &mut Connections) -> Result<()> {
        // TODO(mmk) If we do the comment in add_ec2_instance_ids_to_connections, then we can
        // probably remove this check.
        if connections.instance_ids().is_empty() {
            return Ok(());
        }

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

    async fn build_connections_for_service(
        &self,
        cluster: &str,
        service: &str,
        container: Option<&str>,
    ) -> Result<Connections> {
        let mut connections = self.build_connections(cluster, service).await?;
        self.add_containers_to_connections(cluster, container, &mut connections)
            .await?;
        self.add_ec2_instance_ids_to_connections(cluster, &mut connections)
            .await?;
        self.add_name_and_ip(&mut connections).await?;

        Ok(connections)
    }
}

fn arn_to_id(arn: &str) -> &str {
    arn.split('/').last().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::arn_to_id;

    #[test]
    fn arn_to_id_works_as_expected() {
        assert_eq!(
            "abcdefghijklmnopqrstuvwxyz",
            arn_to_id(
                "arn:aws:ecs:us-east-1:123456789012:task/cluster-name/abcdefghijklmnopqrstuvwxyz"
            )
        );
    }
}
