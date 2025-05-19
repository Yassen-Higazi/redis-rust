use std::net::SocketAddr;

use crate::utils::gen_id;

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum Role {
    Master,
    Slave,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Replica {
    Master {
        id: String,
        address: String,
        slaves: Vec<Replica>,
        replication_offset: i64,
    },

    Slave {
        id: String,
        address: String,
        master_address: SocketAddr,
    },
}

impl Replica {
    pub fn new(role: Role, master_address: Option<SocketAddr>) -> Self {
        match role {
            Role::Master => Self::new_master(),

            Role::Slave => Self::new_slave(master_address),
        }
    }

    pub fn new_master() -> Self {
        Self::Master {
            id: gen_id(),
            replication_offset: 0,
            address: String::from("localhost:6379"),
            slaves: Vec::new(),
        }
    }

    pub fn new_slave(master_address: Option<SocketAddr>) -> Self {
        Self::Slave {
            id: gen_id(),
            master_address: master_address.expect("Master address is required for slave"),
            address: String::from("localhost:6379"),
        }
    }

    pub fn get_replication_status(&self) -> String {
        match self {
            Self::Master {
                id,
                slaves,
                replication_offset,
                ..
            } => format!(
                "# Replication
role:master
connected_slaves:{}
master_replid:{id}
master_repl_offset:{replication_offset}
second_repl_offset:-1
repl_backlog_active:0
repl_backlog_size:1048576
repl_backlog_first_byte_offset:0
repl_backlog_histlen:
                            ",
                slaves.len()
            ),
            Self::Slave { id, .. } => format!(
                "# Replication
role:slave
slave_replid:{id} "
            ),
        }
    }
}
