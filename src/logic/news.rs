use std::io::Read;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::Serialize;
use termimad::MadSkin;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::{
    api,
    commands::news::{
        HotArgs, HotWeekArgs, ListArgs, NewsAction, NewsCommand, RecommendedArgs, SearchArgs,
        ShowArgs,
    },
    context::Context,
    models::news::{NewsDetail, NewsInfo},
};

/// 批量 `news show` 的单条结果：成功包 detail，失败包 error 字符串。
/// `#[serde(tag = "status")]` 使 JSON 形态稳定：`{"status":"ok",...}` / `{"status":"error",...}`
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum DetailResult {
    Ok { id: u64, detail: NewsDetail },
    Error { id: u64, error: String },
}

pub async fn endpoint(cmd: NewsCommand, ctx: &mut Context) -> Result<()> {
    match cmd.commands {
        NewsAction::List(arg) => handle_list(arg, ctx).await,
        NewsAction::Show(arg) => handle_show(arg, ctx).await,
        NewsAction::Search(arg) => handle_search(arg, ctx).await,
        NewsAction::Hot(arg) => handle_hot(arg, ctx).await,
        NewsAction::HotWeek(arg) => handle_hot_week(arg, ctx).await,
        NewsAction::Recommended(arg) => handle_recommended(arg, ctx).await,
    }
}

/// 把一组新闻按 json / ids_only / title_only / 完整 四种形态输出
/// 优先级：`--json` > `--ids-only` > `--title-only` > 完整渲染
/// （多 flag 同时给时按 specificity 裁决，避免 clap 互斥配置膨胀）
fn render_news_list(
    mut list: Vec<NewsInfo>,
    title_only: bool,
    ids_only: bool,
    ctx: &mut Context,
) -> Result<()> {
    if ctx.json {
        // 序列化前回填 Url 派生字段，skill 不必自己拼
        for item in &mut list {
            if item.url.is_none() {
                item.url = Some(item.build_url());
            }
        }
        return ctx.terminal.json(&list);
    }
    if ids_only {
        for n in &list {
            ctx.terminal.writeln(n.id)?;
        }
        return Ok(());
    }
    for (index, news) in list.into_iter().enumerate() {
        if title_only {
            ctx.terminal.writeln(news.into_title_format(index))?;
        } else {
            ctx.terminal.writeln(news.into_format())?;
        }
    }
    Ok(())
}

async fn handle_list(arg: ListArgs, ctx: &mut Context) -> Result<()> {
    let (title_only, ids_only) = (arg.title_only, arg.ids_only);
    let news_list = api::news::list_news(&ctx.client, arg).await?;
    render_news_list(news_list, title_only, ids_only, ctx)
}

async fn handle_hot(mut arg: HotArgs, ctx: &mut Context) -> Result<()> {
    // 三者都没传时默认最近 7 天；显式 --last-days 同样在此翻译成 startDate/endDate
    // clap 已保证 start/end 与 last_days 不会同时出现
    if arg.start_date.is_none() && arg.end_date.is_none() {
        let days = arg.last_days.unwrap_or(7);
        let today = chrono::Local::now().date_naive();
        arg.end_date = Some(today.format("%Y-%m-%d").to_string());
        arg.start_date = Some(
            (today - chrono::Duration::days(days as i64))
                .format("%Y-%m-%d")
                .to_string(),
        );
    }
    let (title_only, ids_only) = (arg.title_only, arg.ids_only);
    let news_list = api::news::hot_news(&ctx.client, arg).await?;
    render_news_list(news_list, title_only, ids_only, ctx)
}

async fn handle_hot_week(arg: HotWeekArgs, ctx: &mut Context) -> Result<()> {
    let (title_only, ids_only) = (arg.title_only, arg.ids_only);
    let news_list = api::news::hot_week_news(&ctx.client, arg).await?;
    render_news_list(news_list, title_only, ids_only, ctx)
}

async fn handle_recommended(arg: RecommendedArgs, ctx: &mut Context) -> Result<()> {
    let (title_only, ids_only) = (arg.title_only, arg.ids_only);
    let news_list = api::news::recommended_news(&ctx.client, arg).await?;
    render_news_list(news_list, title_only, ids_only, ctx)
}

async fn handle_show(arg: ShowArgs, ctx: &mut Context) -> Result<()> {
    let ids = collect_show_ids(&arg)?;
    if ids.is_empty() {
        anyhow::bail!("未提供新闻 ID（位置参数为空且 --stdin 没读到任何 ID）");
    }

    // 单 ID 且不是 --stdin 调用 → 保留原有人类友好渲染（或单条 JSON）
    if ids.len() == 1 && !arg.stdin {
        return render_single_show(ids[0], ctx).await;
    }

    // 多 ID 或 --stdin → 并发拉取，强制输出 JSON 数组（html→md 渲染对批量调用方是噪声）
    let results = fetch_details_concurrent(&ids, &ctx.client, arg.max_concurrent).await;
    let any_ok = results.iter().any(|r| matches!(r, DetailResult::Ok { .. }));
    ctx.terminal.json(&results)?;
    // stdout 已经写出完整结果，skill 仍可解析每条 error；exit code 让快路径用 `$?` 判整批失败
    if !any_ok {
        anyhow::bail!("批量获取失败：{} 条全部失败", results.len());
    }
    Ok(())
}

fn collect_show_ids(arg: &ShowArgs) -> Result<Vec<u64>> {
    if arg.stdin {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        let mut out = Vec::new();
        for tok in buf.split_whitespace() {
            let n: u64 = tok
                .parse()
                .map_err(|_| anyhow!("无效的 ID '{}'，需要正整数", tok))?;
            if n == 0 {
                anyhow::bail!("无效的 ID '0'，需要正整数");
            }
            out.push(n);
        }
        Ok(out)
    } else {
        Ok(arg.ids.clone())
    }
}

async fn render_single_show(id: u64, ctx: &mut Context) -> Result<()> {
    let mut detail = api::news::get_news_detail(&ctx.client, id).await?;

    if ctx.json {
        // skill 总要纯文本版本，序列化前补 MarkdownContent；
        // html2md::parse_html 是同步 CPU 工作，放 spawn_blocking 避免阻塞 runtime worker
        enrich_with_markdown(&mut detail).await;
        ctx.terminal.json(&detail)?;
        return Ok(());
    }

    ctx.terminal.writeln(detail.into_header_format())?;
    let clean_content = detail.clean_content();
    let md = html2md::parse_html(&clean_content);
    let mds = MadSkin::default_light();
    mds.write_text_on(&mut ctx.terminal.stdout, &md)?;
    Ok(())
}

/// 把 `detail.content`（HTML）转成 markdown 写回 `detail.markdown_content`。
/// 在 worker 线程跑同步转换；万一 task panic 也只是让 markdown_content 留 None，不影响 detail 本身
async fn enrich_with_markdown(detail: &mut NewsDetail) {
    let html = detail.content.clone();
    if let Ok(md) = tokio::task::spawn_blocking(move || html2md::parse_html(&html)).await {
        detail.markdown_content = Some(md);
    }
}

/// 并发获取多条新闻详情。`max_concurrent` 用 Semaphore 控制同时在飞的请求数，
/// 避免 skill 一次喂 100 个 ID 时打爆服务器/本地连接池。
/// 单条失败不阻塞其它；JoinError（task panic）也不会让整批崩，返回数组按 `ids` 输入顺序排列。
async fn fetch_details_concurrent(
    ids: &[u64],
    client: &Client,
    max_concurrent: u32,
) -> Vec<DetailResult> {
    let sem = Arc::new(Semaphore::new(max_concurrent as usize));
    let mut set = JoinSet::new();
    for (idx, &id) in ids.iter().enumerate() {
        let client = client.clone();
        let sem = Arc::clone(&sem);
        set.spawn(async move {
            // permit 拿不到（semaphore closed）几乎不可能，发生时把它当作错误返回不影响其它 ID
            let _permit = match sem.acquire().await {
                Ok(p) => p,
                Err(e) => return (idx, id, Err(anyhow!("semaphore closed: {}", e))),
            };
            let r = match api::news::get_news_detail(&client, id).await {
                Ok(mut detail) => {
                    enrich_with_markdown(&mut detail).await;
                    Ok(detail)
                }
                Err(e) => Err(e),
            };
            (idx, id, r)
        });
    }

    let mut tagged: Vec<(usize, u64, Result<NewsDetail>)> = Vec::with_capacity(ids.len());
    while let Some(join_res) = set.join_next().await {
        if let Ok(t) = join_res {
            tagged.push(t);
        }
    }
    tagged.sort_by_key(|(idx, _, _)| *idx);

    tagged
        .into_iter()
        .map(|(_, id, r)| match r {
            Ok(detail) => DetailResult::Ok { id, detail },
            Err(e) => DetailResult::Error {
                id,
                error: e.to_string(),
            },
        })
        .collect()
}

async fn handle_search(arg: SearchArgs, ctx: &mut Context) -> Result<()> {
    let title_only = arg.title_only;
    let results = api::news::search_news(&ctx.client, arg).await?;

    if ctx.json {
        ctx.terminal.json(&results)?;
        return Ok(());
    }

    for (index, doc) in results.into_iter().enumerate() {
        if title_only {
            ctx.terminal.writeln(doc.into_title_format(index))?;
        } else {
            ctx.terminal.writeln(doc.into_format(index))?;
        }
    }

    Ok(())
}

