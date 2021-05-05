use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Debug)]
pub struct Container {
    pub name: String,
    pub runtime_id: String,
}

pub struct Connections {
    connections: HashMap<String, Connection>,
    container_instance_id_to_task_id_map: HashMap<String, String>,
    instance_id_to_task_id_map: HashMap<String, String>,
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
            .filter_map(|(_, connection)| connection.container_instance_id.clone())
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
            .insert(container_instance_id.clone(), task_id.clone());
        self.connections
            .get_mut(&task_id)
            .unwrap()
            .set_container_instance_id(container_instance_id);
    }

    pub fn set_ec2_instance_id(&mut self, container_instance_id: String, ec2_instance_id: String) {
        let task_id = self
            .container_instance_id_to_task_id_map
            .get(&container_instance_id)
            .unwrap();

        self.instance_id_to_task_id_map
            .insert(ec2_instance_id.clone(), task_id.to_string());

        self.connections
            .get_mut(&task_id.clone())
            .unwrap()
            .set_instance_id(ec2_instance_id);
    }

    pub fn set_name_and_ip(&mut self, ec2_instance_id: String, name: String, ip: String) {
        let task_id = self
            .instance_id_to_task_id_map
            .get(&ec2_instance_id)
            .unwrap();

        self.connections
            .get_mut(&task_id.clone())
            .unwrap()
            .set_name_and_ip(name, ip);
    }

    pub fn get_connections(&self) -> Vec<Connection> {
        self.connections
            .clone()
            .into_iter()
            .map(|(_, connection)| connection)
            .collect()
    }

    pub fn get_connection_choices(&self) -> Vec<ConnectionChoice> {
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

    fn get_connection_choices(&self) -> Vec<ConnectionChoice> {
        self.containers
            .iter()
            .map(|container| ConnectionChoice {
                instance_id: self.instance_id.clone().unwrap(),
                instance_name: self.instance_name.clone().unwrap(),
                private_ip: self.private_ip.clone().unwrap(),
                name: container.name.clone(),
                runtime_id: container.runtime_id.clone(),
            })
            .collect()
    }
}

#[derive(Debug)]
// TODO(mmk) Do we really need to expose all of these as public fields?
pub struct ConnectionChoice {
    pub instance_id: String,
    pub instance_name: String,
    pub private_ip: String,
    pub name: String,
    pub runtime_id: String,
}

impl fmt::Display for ConnectionChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}) on {} ({})",
            self.name, self.runtime_id, self.instance_name, self.instance_id
        )
    }
}
