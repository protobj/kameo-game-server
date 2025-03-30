use crate::node::Node;
use kameo::{Actor, Reply};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

mod login_handler;
mod router;
pub(crate) mod node;