use anyhow::Result;
use reqwest::{Client, Response};
use serde::Serialize;

use crate::{
    api::urls::OPENAPI,
    models::news::{NewsInfo, NewsDetail, ZzkDocument},
    tools::IntoAnyhowResult,
};

pub async fn list_news(c: &Client, page: impl Serialize + Send + Sync) -> Result<Vec<NewsInfo>> {
    let resp = raw_list_news(c, page).await?;
    resp.error_for_status()?
        .json()
        .await
        .into_anyhow_result()
}

pub async fn raw_list_news(c: &Client, page: impl Serialize + Send + Sync) -> Result<Response> {
    let url = format!("{}/{}", OPENAPI, "newsitems");
    c.get(url)
    .query(&page)
    .send()
    .await
    .into_anyhow_result()
}

pub async fn get_news_detail(c: &Client, id: u64) -> Result<NewsDetail> {
    let resp = raw_get_news_detail(c, id).await?;
    let status = resp.status();
    // 先把 body 落地再判断 status，否则 error_for_status() 会丢掉 body，
    // 失败时拿不到服务器返回的具体信息（不存在的 newsid 常返回 200 + 非 NewsDetail JSON）
    let body = resp.text().await.into_anyhow_result()?;

    if !status.is_success() {
        anyhow::bail!(
            "获取新闻详情失败 (id={id}, HTTP {code}): {body}",
            id = id,
            code = status.as_u16(),
            body = truncate_for_error(&body, 200),
        );
    }

    // cnblogs 对不存在的 newsid 返回 200 + NewsId=0/Title=null 的空架子，
    // 识别后给批量调用方一个稳定的"新闻不存在"错误，避免靠 body preview 反推
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
        let news_id_zero = v.get("NewsId").and_then(|x| x.as_u64()) == Some(0);
        let title_null = v.get("Title").is_some_and(|x| x.is_null());
        if news_id_zero && title_null {
            anyhow::bail!("新闻不存在 (id={id})", id = id);
        }
    }

    serde_json::from_str::<NewsDetail>(&body).map_err(|e| {
        anyhow::anyhow!(
            "解析新闻详情失败 (id={id}, HTTP {code}): {err}; body preview: {body}",
            id = id,
            code = status.as_u16(),
            err = e,
            body = truncate_for_error(&body, 200),
        )
    })
}

fn truncate_for_error(s: &str, max_chars: usize) -> String {
    let total = s.chars().count();
    if total <= max_chars {
        s.to_string()
    } else {
        format!(
            "{}…(total {} chars)",
            s.chars().take(max_chars).collect::<String>(),
            total
        )
    }
}

pub async fn raw_get_news_detail(c: &Client, id: u64) -> Result<Response> {
    let url = format!("{}/newsitems/{}", OPENAPI, id);
    c.get(url).send().await.into_anyhow_result()
}

pub async fn search_news(c: &Client, params: impl Serialize + Send + Sync) -> Result<Vec<ZzkDocument>> {
    let resp = raw_search_news(c, params).await?;
    resp.error_for_status()?
        .json()
        .await
        .into_anyhow_result()
}

pub async fn raw_search_news(c: &Client, params: impl Serialize + Send + Sync) -> Result<Response> {
    let url = format!("{}/ZzkDocuments/News", OPENAPI);
    c.get(url)
        .query(&params)
        .send()
        .await
        .into_anyhow_result()
}

pub async fn hot_news(c: &Client, params: impl Serialize + Send + Sync) -> Result<Vec<NewsInfo>> {
    let resp = raw_hot_news(c, params).await?;
    resp.error_for_status()?
        .json()
        .await
        .into_anyhow_result()
}

pub async fn raw_hot_news(c: &Client, params: impl Serialize + Send + Sync) -> Result<Response> {
    let url = format!("{}/newsitems/@hot", OPENAPI);
    c.get(url).query(&params).send().await.into_anyhow_result()
}

pub async fn hot_week_news(c: &Client, params: impl Serialize + Send + Sync) -> Result<Vec<NewsInfo>> {
    let resp = raw_hot_week_news(c, params).await?;
    resp.error_for_status()?
        .json()
        .await
        .into_anyhow_result()
}

pub async fn raw_hot_week_news(c: &Client, params: impl Serialize + Send + Sync) -> Result<Response> {
    let url = format!("{}/newsitems/@hot-week", OPENAPI);
    c.get(url).query(&params).send().await.into_anyhow_result()
}

pub async fn recommended_news(c: &Client, params: impl Serialize + Send + Sync) -> Result<Vec<NewsInfo>> {
    let resp = raw_recommended_news(c, params).await?;
    resp.error_for_status()?
        .json()
        .await
        .into_anyhow_result()
}

pub async fn raw_recommended_news(c: &Client, params: impl Serialize + Send + Sync) -> Result<Response> {
    let url = format!("{}/newsitems/@recommended", OPENAPI);
    c.get(url).query(&params).send().await.into_anyhow_result()
}
