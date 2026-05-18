use anyhow::Result;
use termimad::MadSkin;

use crate::{
    api,
    commands::news::{ListArgs, NewsAction, NewsCommand, SearchArgs, ShowArgs},
    context::Context,
};

pub async fn endpoint(cmd: NewsCommand, ctx: &mut Context) -> Result<()> {
    match cmd.commands {
        NewsAction::List(arg) => handle_list(arg, ctx).await,
        NewsAction::Show(arg) => handle_show(arg, ctx).await,
        NewsAction::Search(arg) => handle_search(arg, ctx).await,
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
    let detail = api::news::get_news_detail(&ctx.client, arg.id).await?;

    if ctx.json {
        ctx.terminal.json(&detail)?;
        return Ok(());
    }

    // 显示标题栏
    ctx.terminal.writeln(detail.into_header_format())?;

    // 转换HTML为Markdown并在终端渲染
    let clean_content = detail.clean_content();
    let md = html2md::parse_html(&clean_content);
    let mds = MadSkin::default_light();
    mds.write_text_on(&mut ctx.terminal.stdout, &md)?;
    Ok(())
}

async fn handle_search(arg: SearchArgs, ctx: &mut Context) -> Result<()> {
    let results = api::news::search_news(&ctx.client, arg).await?;

    if ctx.json {
        ctx.terminal.json(&results)?;
        return Ok(());
    }

    for (index, doc) in results.into_iter().enumerate() {
        ctx.terminal.writeln(doc.into_format(index))?;
    }

    Ok(())
}

