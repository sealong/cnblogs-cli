use std::sync::Arc;

use anyhow::{Result, anyhow};
use reqwest::Client;
use rmcp::{
    ErrorData as McpError, ServiceExt, handler::server::wrapper::Parameters, model::CallToolResult,
    tool, tool_router, transport::stdio,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::{sync::Semaphore, task::JoinSet};

use crate::commands::news::{
    HotArgs, HotWeekArgs, ListArgs, RecommendedArgs, SearchArgs, SearchSort, ShowArgs,
};
use crate::{
    api,
    context::Context,
    mcp::post::{
        ListPostParams, SearchPostParams, ShowPostParams, list_posts_json, search_posts_json,
        show_post_json,
    },
    models::news::{NewsDetail, NewsInfo, ZzkDocument, strip_html},
};

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct ListNewsParams {
    /// 分页页码，从 1 开始。
    pub page_index: Option<u64>,
    /// 每页新闻条数，默认 10。
    pub page_size: Option<u64>,
}

impl ListNewsParams {
    pub fn into_list_args(self) -> ListArgs {
        ListArgs {
            page_index: self.page_index.unwrap_or(1),
            page_size: self.page_size.unwrap_or(10),
            title_only: false,
            ids_only: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NewsSearchSort {
    Relevance,
    Time,
}

impl From<NewsSearchSort> for SearchSort {
    fn from(value: NewsSearchSort) -> Self {
        match value {
            NewsSearchSort::Relevance => SearchSort::Relevance,
            NewsSearchSort::Time => SearchSort::Time,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchNewsParams {
    /// 搜索关键词，不能为空。
    pub keywords: String,
    /// 分页页码，从 1 开始。
    pub page_index: Option<u64>,
    /// 开始日期，格式 YYYY-MM-DD。
    pub start_date: Option<String>,
    /// 结束日期，格式 YYYY-MM-DD。
    pub end_date: Option<String>,
    /// 最低浏览次数。
    pub min_views: Option<u64>,
    /// 排序方式：relevance 按搜索相关度，time 按发布时间倒序。
    pub sort: Option<NewsSearchSort>,
    /// 返回条数，范围 1..=15，默认 15。
    pub limit: Option<u32>,
}

impl SearchNewsParams {
    pub fn into_search_args(self) -> Result<SearchArgs> {
        if self.keywords.trim().is_empty() {
            return Err(anyhow!("keywords 不能为空"));
        }

        let limit = self.limit.unwrap_or(15);
        if !(1..=15).contains(&limit) {
            return Err(anyhow!("limit 必须在 1..=15 之间"));
        }

        Ok(SearchArgs {
            keywords: self.keywords,
            page_index: self.page_index.unwrap_or(1),
            start_date: self.start_date,
            end_date: self.end_date,
            view_times_at_least: self.min_views,
            title_only: false,
            ids_only: false,
            sort: self.sort.unwrap_or(NewsSearchSort::Relevance).into(),
            limit,
        })
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HotNewsParams {
    /// 最近 N 天热门新闻；不能与 start_date/end_date 同时使用。
    pub last_days: Option<u32>,
    /// 开始日期，格式 YYYY-MM-DD；必须与 end_date 成对提供。
    pub start_date: Option<String>,
    /// 结束日期，格式 YYYY-MM-DD；必须与 start_date 成对提供。
    pub end_date: Option<String>,
    /// 分页页码，从 1 开始。
    pub page_index: Option<u64>,
    /// 每页新闻条数，默认 10。
    pub page_size: Option<u64>,
}

impl HotNewsParams {
    pub fn into_hot_args(self) -> Result<HotArgs> {
        if self.last_days.is_some() && (self.start_date.is_some() || self.end_date.is_some()) {
            return Err(anyhow!("last_days 不能与 start_date/end_date 同时使用"));
        }
        if self.start_date.is_some() != self.end_date.is_some() {
            return Err(anyhow!("start_date 和 end_date 必须成对提供"));
        }

        Ok(HotArgs {
            start_date: self.start_date,
            end_date: self.end_date,
            last_days: self.last_days,
            page_index: self.page_index.unwrap_or(1),
            page_size: self.page_size.unwrap_or(10),
            title_only: false,
            ids_only: false,
        })
    }
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct HotWeekNewsParams {
    /// 分页页码，从 1 开始。
    pub page_index: Option<u64>,
    /// 每页新闻条数，默认 10。
    pub page_size: Option<u64>,
}

impl HotWeekNewsParams {
    pub fn into_hot_week_args(self) -> HotWeekArgs {
        HotWeekArgs {
            page_index: self.page_index.unwrap_or(1),
            page_size: self.page_size.unwrap_or(10),
            title_only: false,
            ids_only: false,
        }
    }
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct RecommendedNewsParams {
    /// 分页页码，从 1 开始。
    pub page_index: Option<u64>,
    /// 每页新闻条数，默认 10。
    pub page_size: Option<u64>,
}

impl RecommendedNewsParams {
    pub fn into_recommended_args(self) -> RecommendedArgs {
        RecommendedArgs {
            page_index: self.page_index.unwrap_or(1),
            page_size: self.page_size.unwrap_or(10),
            title_only: false,
            ids_only: false,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShowNewsParams {
    /// 新闻 ID 列表。已知 ID 时优先使用此参数。
    pub ids: Option<Vec<u64>>,
    /// 新闻标题。用户只提供标题时使用此参数，服务会先解析出新闻 ID 再获取详情。
    pub title: Option<String>,
    /// 批量请求并发上限，范围 1..=64，默认 8。
    pub max_concurrent: Option<u32>,
    /// 是否保留 HTML 原文；默认 false，只返回 MarkdownContent。
    pub include_html: Option<bool>,
}

impl ShowNewsParams {
    pub fn normalized_title(&self) -> Option<String> {
        self.title
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
    }

    pub fn validate_lookup(&self) -> Result<()> {
        let has_ids = self.ids.as_ref().is_some_and(|ids| !ids.is_empty());
        let has_title = self.normalized_title().is_some();
        if !has_ids && !has_title {
            return Err(anyhow!("ids 或 title 至少提供一个"));
        }
        if self
            .ids
            .as_ref()
            .is_some_and(|ids| ids.iter().any(|id| *id == 0))
        {
            return Err(anyhow!("ids 不能包含 0"));
        }
        Ok(())
    }

    pub fn into_show_args(self) -> Result<ShowArgs> {
        self.validate_lookup()?;
        let ids = self.ids.unwrap_or_default();
        if ids.is_empty() {
            return Err(anyhow!("ids 不能为空"));
        }

        let max_concurrent = self.max_concurrent.unwrap_or(8);
        if !(1..=64).contains(&max_concurrent) {
            return Err(anyhow!("max_concurrent 必须在 1..=64 之间"));
        }

        Ok(ShowArgs {
            ids,
            stdin: false,
            max_concurrent,
            include_html: self.include_html.unwrap_or(false),
        })
    }
}

#[derive(Clone)]
pub struct NewsMcpServer {
    client: Client,
    blog_app: String,
}

impl NewsMcpServer {
    pub fn new(client: Client, blog_app: String) -> Self {
        Self { client, blog_app }
    }
}

#[tool_router(server_handler)]
impl NewsMcpServer {
    #[tool(description = "List the latest cnblogs news items as JSON.")]
    async fn cnb_news_list(
        &self,
        Parameters(params): Parameters<ListNewsParams>,
    ) -> Result<CallToolResult, McpError> {
        list_news_json(&self.client, params)
            .await
            .map_err(mcp_error)
    }

    #[tool(description = "Search cnblogs news by keyword as JSON.")]
    async fn cnb_news_search(
        &self,
        Parameters(params): Parameters<SearchNewsParams>,
    ) -> Result<CallToolResult, McpError> {
        search_news_json(&self.client, params)
            .await
            .map_err(mcp_error)
    }

    #[tool(
        description = "Fetch cnblogs news details as JSON. Use ids when known; use title when the user gives a news title but no ID."
    )]
    async fn cnb_news_show(
        &self,
        Parameters(params): Parameters<ShowNewsParams>,
    ) -> Result<CallToolResult, McpError> {
        show_news_json(&self.client, params)
            .await
            .map_err(mcp_error)
    }

    #[tool(description = "List hot cnblogs news for a relative or absolute date range as JSON.")]
    async fn cnb_news_hot(
        &self,
        Parameters(params): Parameters<HotNewsParams>,
    ) -> Result<CallToolResult, McpError> {
        hot_news_json(&self.client, params).await.map_err(mcp_error)
    }

    #[tool(description = "List this week's hot cnblogs news as JSON.")]
    async fn cnb_news_hot_week(
        &self,
        Parameters(params): Parameters<HotWeekNewsParams>,
    ) -> Result<CallToolResult, McpError> {
        hot_week_news_json(&self.client, params)
            .await
            .map_err(mcp_error)
    }

    #[tool(description = "List recommended cnblogs news as JSON.")]
    async fn cnb_news_recommended(
        &self,
        Parameters(params): Parameters<RecommendedNewsParams>,
    ) -> Result<CallToolResult, McpError> {
        recommended_news_json(&self.client, params)
            .await
            .map_err(mcp_error)
    }

    #[tool(
        description = "List cnblogs blog posts for a blog_app as JSON. If blog_app is omitted, the current logged-in blog is used."
    )]
    async fn cnb_post_list(
        &self,
        Parameters(params): Parameters<ListPostParams>,
    ) -> Result<CallToolResult, McpError> {
        list_posts_json(&self.client, &self.blog_app, params)
            .await
            .map_err(mcp_error)
    }

    #[tool(description = "Search all cnblogs blog posts by keyword as JSON.")]
    async fn cnb_post_search(
        &self,
        Parameters(params): Parameters<SearchPostParams>,
    ) -> Result<CallToolResult, McpError> {
        search_posts_json(&self.client, params)
            .await
            .map_err(mcp_error)
    }

    #[tool(
        description = "Fetch a cnblogs blog post body as JSON with MarkdownContent. Set include_html to true to also return the HTML body."
    )]
    async fn cnb_post_show(
        &self,
        Parameters(params): Parameters<ShowPostParams>,
    ) -> Result<CallToolResult, McpError> {
        show_post_json(&self.client, params)
            .await
            .map_err(mcp_error)
    }
}

pub async fn serve_stdio(timeout: Option<u64>) -> Result<()> {
    let ctx = Context::new(timeout)?;
    let service = NewsMcpServer::new(ctx.client, ctx.cache.blog_app)
        .serve(stdio())
        .await?;
    service.waiting().await?;
    Ok(())
}

async fn list_news_json(client: &Client, params: ListNewsParams) -> Result<CallToolResult> {
    let mut list = api::news::list_news(client, params.into_list_args()).await?;
    add_news_urls(&mut list);
    to_tool_items_result(&list)
}

async fn search_news_json(client: &Client, params: SearchNewsParams) -> Result<CallToolResult> {
    let args = params.into_search_args()?;
    let sort = args.sort;
    let limit = args.limit;
    let mut results = api::news::search_news(client, args).await?;
    clean_search_results(&mut results);
    if matches!(sort, SearchSort::Time) {
        results.sort_by(|a, b| b.publish_time.cmp(&a.publish_time));
    }
    results.truncate(limit as usize);
    to_tool_items_result(&results)
}

async fn hot_news_json(client: &Client, params: HotNewsParams) -> Result<CallToolResult> {
    let mut args = params.into_hot_args()?;
    if args.start_date.is_none() && args.end_date.is_none() {
        let days = args.last_days.unwrap_or(7);
        let today = chrono::Local::now().date_naive();
        args.end_date = Some(today.format("%Y-%m-%d").to_string());
        args.start_date = Some(
            (today - chrono::Duration::days(days as i64))
                .format("%Y-%m-%d")
                .to_string(),
        );
    }

    let mut list = api::news::hot_news(client, args).await?;
    add_news_urls(&mut list);
    to_tool_items_result(&list)
}

async fn hot_week_news_json(client: &Client, params: HotWeekNewsParams) -> Result<CallToolResult> {
    let mut list = api::news::hot_week_news(client, params.into_hot_week_args()).await?;
    add_news_urls(&mut list);
    to_tool_items_result(&list)
}

async fn recommended_news_json(
    client: &Client,
    params: RecommendedNewsParams,
) -> Result<CallToolResult> {
    let mut list = api::news::recommended_news(client, params.into_recommended_args()).await?;
    add_news_urls(&mut list);
    to_tool_items_result(&list)
}

async fn show_news_json(client: &Client, params: ShowNewsParams) -> Result<CallToolResult> {
    params.validate_lookup()?;

    let max_concurrent = params.max_concurrent.unwrap_or(8);
    if !(1..=64).contains(&max_concurrent) {
        return Err(anyhow!("max_concurrent 必须在 1..=64 之间"));
    }

    let include_html = params.include_html.unwrap_or(false);
    let ids = match params.ids {
        Some(ids) if !ids.is_empty() => ids,
        _ => vec![
            resolve_news_id_by_title(
                client,
                &params
                    .normalized_title()
                    .ok_or_else(|| anyhow!("ids 或 title 至少提供一个"))?,
            )
            .await?,
        ],
    };

    let results = fetch_details_concurrent(&ids, client, max_concurrent, include_html).await;
    let any_ok = results.iter().any(|r| matches!(r, DetailResult::Ok { .. }));
    if !any_ok {
        return Err(anyhow!("批量获取失败：{} 条全部失败", results.len()));
    }
    to_tool_items_result(&results)
}

async fn resolve_news_id_by_title(client: &Client, title: &str) -> Result<u64> {
    let target = normalize_title_for_match(title);

    let recommended = api::news::recommended_news(
        client,
        RecommendedArgs {
            page_index: 1,
            page_size: 20,
            title_only: false,
            ids_only: false,
        },
    )
    .await?;
    if let Some(id) = find_news_info_id_by_title(&recommended, &target) {
        return Ok(id);
    }

    let latest = api::news::list_news(
        client,
        ListArgs {
            page_index: 1,
            page_size: 20,
            title_only: false,
            ids_only: false,
        },
    )
    .await?;
    if let Some(id) = find_news_info_id_by_title(&latest, &target) {
        return Ok(id);
    }

    let mut results = api::news::search_news(
        client,
        SearchArgs {
            keywords: title.to_string(),
            page_index: 1,
            start_date: None,
            end_date: None,
            view_times_at_least: None,
            title_only: false,
            ids_only: false,
            sort: SearchSort::Relevance,
            limit: 15,
        },
    )
    .await?;
    clean_search_results(&mut results);

    if let Some(id) = results
        .iter()
        .find(|doc| normalize_title_for_match(&doc.title) == target)
        .and_then(ZzkDocument::news_id)
    {
        return Ok(id);
    }

    results
        .iter()
        .find_map(ZzkDocument::news_id)
        .ok_or_else(|| anyhow!("未找到标题匹配的新闻：{}", title))
}

fn find_news_info_id_by_title(list: &[NewsInfo], target: &str) -> Option<u64> {
    list.iter()
        .find(|item| normalize_title_for_match(&item.title) == target)
        .map(|item| item.id)
}

fn normalize_title_for_match(title: &str) -> String {
    title.trim().replace(char::is_whitespace, "")
}

fn add_news_urls(list: &mut [NewsInfo]) {
    for item in list {
        if item.url.is_none() {
            item.url = Some(item.build_url());
        }
    }
}

fn clean_search_results(results: &mut [ZzkDocument]) {
    for doc in results {
        doc.title = strip_html(&doc.title);
        doc.content = strip_html(&doc.content);
    }
}

async fn enrich_with_markdown(detail: &mut NewsDetail) {
    let html = detail.content.clone();
    if let Ok(md) = tokio::task::spawn_blocking(move || html2md::parse_html(&html)).await {
        detail.markdown_content = Some(md);
    }
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum DetailResult {
    Ok { id: u64, detail: NewsDetail },
    Error { id: u64, error: String },
}

async fn fetch_details_concurrent(
    ids: &[u64],
    client: &Client,
    max_concurrent: u32,
    include_html: bool,
) -> Vec<DetailResult> {
    let sem = Arc::new(Semaphore::new(max_concurrent as usize));
    let mut set = JoinSet::new();
    for (idx, &id) in ids.iter().enumerate() {
        let client = client.clone();
        let sem = Arc::clone(&sem);
        set.spawn(async move {
            let _permit = match sem.acquire().await {
                Ok(p) => p,
                Err(e) => return (idx, id, Err(anyhow!("semaphore closed: {}", e))),
            };

            let r = match api::news::get_news_detail(&client, id).await {
                Ok(mut detail) => {
                    enrich_with_markdown(&mut detail).await;
                    if !include_html {
                        detail.content.clear();
                    }
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

fn to_tool_result<T: Serialize>(value: &T) -> Result<CallToolResult> {
    Ok(CallToolResult::structured(serde_json::to_value(value)?))
}

fn to_tool_items_result<T: Serialize>(items: &T) -> Result<CallToolResult> {
    #[derive(Serialize)]
    struct Items<'a, T> {
        items: &'a T,
    }

    to_tool_result(&Items { items })
}

fn mcp_error(error: anyhow::Error) -> McpError {
    McpError::internal_error(error.to_string(), None)
}
