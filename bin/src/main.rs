use common::config::{ARGS, Args, GlobalConfig, init_config};
use common::logging::init_log;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    
    //1.初始化命令行参数
    init_config();
    let args = match ARGS.get() {
        None => return Err(anyhow::anyhow!("args error")),
        Some(x) => x,
    };
    
    //2.读配置文件
    let config: GlobalConfig = <&Args as Into<anyhow::Result<GlobalConfig>>>::into(args)?;

    //3.启动日志输出
    let log_name = if args.server.len() > 1 {
        String::from("all")
    } else if args.server.len() <= 0 {
        return Err(anyhow::anyhow!("set server error"));
    } else {
        let x = args.server.get(0).unwrap();
        x.to_string()
    };
    let log_guards = init_log(config.log.clone(), log_name)?;

    tracing::info!("config:{:?}", config);

    tracing::trace!("log:{:?}", log_guards);
    Ok(())
}
