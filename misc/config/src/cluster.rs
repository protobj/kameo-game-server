use clap::ValueEnum;

#[derive(Debug)]
pub struct Cluster {
    pub id: u32,
    pub role: Role,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Role {
    #[value(name = "Login")]
    Login,
    #[value(name = "Gate")]
    Gate,
    #[value(name = "Game")]
    Game,
    #[value(name = "World")]
    World,
}
pub fn get_server_id(role: &Role, id: &u32) -> String {
    return format!("{:?}/{}", role, id);
}
