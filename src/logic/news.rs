use anyhow::Result;
use termimad::MadSkin;

use crate::{
    api,
    commands::news::{ListArgs, NewsAction, NewsCommand, ShowArgs},
    context::Context,
};

pub async fn endpoint(cmd: NewsCommand, ctx: &mut Context) -> Result<()> {
    match cmd.commands {
        NewsAction::List(arg) => handle_list(arg, ctx).await,
        NewsAction::Show(arg) => handle_show(arg, ctx).await,
    }
}

async fn handle_list(arg: ListArgs, ctx: &mut Context) -> Result<()> {
    let title_only = arg.title_only;
    let news_list = api::news::list_news(&ctx.client, arg).await?;

    for (index, news) in news_list.into_iter().enumerate() {
        if title_only {
            ctx.terminal.writeln(news.into_title_format(index))?;
        } else {
            ctx.terminal.writeln(news.into_format())?;
        }
    }

    Ok(())
}

async fn handle_show(arg: ShowArgs, ctx: &mut Context) -> Result<()> {
    let resp = api::news::get_news_detail(&ctx.client, arg.id).await?;

    // 转换HTML为Markdown并在终端渲染
    let md = html2md::parse_html(&resp);
    let mds = MadSkin::default_light();
    mds.write_text_on(&mut ctx.terminal.stdout, &md)?;
    Ok(())
}

