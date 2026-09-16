use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "cnb-mcp", about = "博客园新闻和博客文章 MCP 服务", version)]
struct Args {
    /// 全局请求超时时间（秒）
    #[arg(long)]
    timeout: Option<u64>,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    cnblogs_lib::mcp::news::serve_stdio(args.timeout).await
}
