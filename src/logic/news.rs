use anyhow::Result;

use crate::{
    api,
    commands::news::{ListArgs, NewsAction, NewsCommand},
    context::Context,
};

pub async fn endpoint(cmd: NewsCommand, ctx: &mut Context) -> Result<()> {
    match cmd.commands {
        NewsAction::List(arg) => handle_list(arg, ctx).await,
    }
}

async fn handle_list(arg: ListArgs, ctx: &mut Context) -> Result<()> {
    let title_only = arg.title_only;
    let news_list = api::news::list_news(&ctx.client, arg).await?;
    
    for news in news_list {
        if title_only {
            ctx.terminal.writeln(news.title)?;
        } else {
            ctx.terminal.writeln(news.into_format())?;
        }
    }
    
    Ok(())
}
