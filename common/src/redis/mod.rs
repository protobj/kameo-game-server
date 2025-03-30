//设计一个数据存储结构,向redis存储UserInfo,用hash表实现,将结构体的字段作为key,字段的值作为value,并且字段可能也是结构体,存储时用二进制序列化方式,要保证版本兼容,用rust实现

use crate::config;
use redis::aio::ConnectionManager;
use redis::ConnectionAddr::Tcp;
use redis::ProtocolVersion::RESP3;
use redis::{ConnectionInfo, RedisConnectionInfo};

//创建一个redis连接池
pub async fn create(conf: &config::RedisConfig) -> anyhow::Result<ConnectionManager> {
    let client = redis::Client::open(ConnectionInfo {
        addr: Tcp(conf.host.to_owned(), conf.port),
        redis: RedisConnectionInfo {
            db: conf.db,
            username: None,
            password: conf.password.to_owned(),
            protocol: RESP3,
        },
    })?;
    Ok(ConnectionManager::new(client).await?)
}
