use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Debug)]
pub struct Container {
    pub name: String,
    pub runtime_id: String,
}

pub struct Connections {
    connections: HashMap<String, Connection>,
    container_instance_id_to_task_id_map: HashMap<String, Vec<String>>,
    instance_id_to_task_id_map: HashMap<String, Vec<String>>,
}

impl Connections {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            container_instance_id_to_task_id_map: HashMap::new(),
            instance_id_to_task_id_map: HashMap::new(),
        }
    }

    pub fn add_connection(&mut self, task_id: String, connection: Connection) {
        self.connections.insert(task_id, connection);
    }

    pub fn task_ids(&self) -> Vec<String> {
        self.connections.keys().cloned().collect()
    }

    pub fn instance_ids(&self) -> Vec<String> {
        self.instance_id_to_task_id_map.keys().cloned().collect()
    }

    pub fn container_arns(&self) -> Vec<String> {
        self.connections
            .clone()
            .into_iter()
            .filter_map(|(_, connection)| connection.container_instance_id)
            .collect()
    }

    pub fn add_container(&mut self, task_id: String, container: Container) {
        self.connections
            .get_mut(&task_id)
            .unwrap()
            .add_container(container);
    }

    pub fn set_container_instance_id(&mut self, task_id: String, container_instance_id: String) {
        self.container_instance_id_to_task_id_map
            .entry(container_instance_id.clone())
            .or_insert(Vec::new())
            .push(task_id.clone());
        self.connections
            .get_mut(&task_id)
            .unwrap()
            .set_container_instance_id(container_instance_id);
    }

    pub fn set_ec2_instance_id(&mut self, container_instance_id: String, ec2_instance_id: String) {
        let task_ids = self
            .container_instance_id_to_task_id_map
            .get(&container_instance_id)
            .unwrap();

        self.instance_id_to_task_id_map
            .insert(ec2_instance_id.clone(), task_ids.clone());

        for task_id in task_ids {
            self.connections
                .get_mut(&task_id.clone())
                .unwrap()
                .set_instance_id(ec2_instance_id.clone());
        }
    }

    pub fn set_name_and_ip(&mut self, ec2_instance_id: String, name: String, ip: String) {
        let task_ids = self
            .instance_id_to_task_id_map
            .get(&ec2_instance_id)
            .unwrap();

        for task_id in task_ids {
            self.connections
                .get_mut(&task_id.clone())
                .unwrap()
                .set_name_and_ip(name.clone(), ip.clone());
        }
    }

    pub fn get_connections(&self) -> Vec<Connection> {
        self.connections
            .clone()
            .into_iter()
            .map(|(_, connection)| connection)
            .collect()
    }

    pub fn get_connection_choices(&self) -> Vec<Box<dyn SshConnection>> {
        self.connections
            .clone()
            .into_iter()
            .flat_map(|(_, connection)| connection.get_connection_choices())
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct Connection {
    container_instance_id: Option<String>,
    containers: Vec<Container>,
    instance_id: Option<String>,
    instance_name: Option<String>,
    private_ip: Option<String>,
}

impl Connection {
    pub fn new() -> Self {
        Self {
            container_instance_id: None,
            containers: Vec::new(),
            instance_id: None,
            instance_name: None,
            private_ip: None,
        }
    }

    pub fn get_containers(&self) -> Vec<Container> {
        self.containers.clone()
    }

    fn add_container(&mut self, container: Container) {
        self.containers.push(container);
    }

    fn set_container_instance_id(&mut self, container_instance_id: String) {
        self.container_instance_id = Some(container_instance_id);
    }

    fn set_instance_id(&mut self, instance_id: String) {
        self.instance_id = Some(instance_id);
    }

    fn set_name_and_ip(&mut self, name: String, ip: String) {
        self.instance_name = Some(name);
        self.private_ip = Some(ip);
    }

    fn get_connection_choices(&self) -> Vec<Box<dyn SshConnection>> {
        self.containers
            .iter()
            .map(|container| {
                Box::new(ContainerChoice {
                    instance_id: self.instance_id.clone().unwrap(),
                    instance_name: self.instance_name.clone().unwrap(),
                    private_ip: self.private_ip.clone().unwrap(),
                    name: container.name.clone(),
                    runtime_id: container.runtime_id.clone(),
                }) as Box<dyn SshConnection>
            })
            .collect()
    }
}

pub trait SshConnection: fmt::Display {
    fn connection(
        &self,
        dns_name: String,
        bastion_port: String,
        bastion_user: String,
        ec2_user: String,
        ssh_identity_file: String,
        cmd: Vec<String>,
    ) -> String;
}

#[derive(Debug)]
// TODO(mmk) Do we really need to expose all of these as public fields?
pub struct ContainerChoice {
    pub instance_id: String,
    pub instance_name: String,
    pub private_ip: String,
    pub name: String,
    pub runtime_id: String,
}

impl fmt::Display for ContainerChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}) on {} ({})",
            self.name, self.runtime_id, self.instance_name, self.instance_id
        )
    }
}

impl SshConnection for ContainerChoice {
    fn connection(
        &self,
        dns_name: String,
        bastion_port: String,
        bastion_user: String,
        ec2_user: String,
        ssh_identity_file: String,
        cmd: Vec<String>,
    ) -> String {
        format!(
            "ssh -i {identity_file} -p {bastion_port} -A -t {bastion_user}@{dns_name} \"ssh -A -t {ec2_user}@{ip} \\\"docker exec -it --detach-keys 'ctrl-q,q' {docker_id} {cmd}\\\"\"",
            identity_file=ssh_identity_file,
            bastion_port=bastion_port,
            bastion_user=bastion_user,
            ec2_user=ec2_user,
            dns_name=dns_name,
            ip=self.private_ip,
            docker_id=&self.runtime_id[..12],
            cmd=cmd.join(" ")
        )
    }
}

pub struct HostConnection {
    pub name: String,
    pub private_ip: String,
    pub instance_id: String,
}

impl fmt::Display for HostConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}) @ {}",
            self.name, self.instance_id, self.private_ip
        )
    }
}

impl SshConnection for HostConnection {
    fn connection(
        &self,
        dns_name: String,
        bastion_port: String,
        bastion_user: String,
        ec2_user: String,
        ssh_identity_file: String,
        cmd: Vec<String>,
    ) -> String {
        format!(
            "ssh -i {identity_file} -p {bastion_port} -A -t {bastion_user}@{dns_name} ssh -A -t {ec2_user}@{ip} {cmd}",
            identity_file=ssh_identity_file,
            bastion_port=bastion_port,
            bastion_user=bastion_user,
            ec2_user=ec2_user,
            dns_name=dns_name,
            ip=self.private_ip,
            cmd=cmd.join(" ")
        )
    }
}
