mod hash;

use r2d2::Pool;
use redis::ConnectionAddr::Tcp;
use redis::ProtocolVersion::RESP3;
use redis::{Commands, ConnectionInfo, RedisConnectionInfo, RedisResult};
//创建一个redis连接池
fn create(conf: &config::RedisConfig) -> anyhow::Result<Pool<redis::Client>> {
    let client = redis::Client::open(ConnectionInfo {
        addr: Tcp(conf.host.to_owned(), conf.port),
        redis: RedisConnectionInfo {
            db: conf.db,
            username: None,
            password: conf.password.to_owned(),
            protocol: RESP3,
        },
    })?;

    let pool = r2d2::Pool::builder()
        .max_size(conf.pool_size)
        .build(client)?;
    Ok(pool)
}
