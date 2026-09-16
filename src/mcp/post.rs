use anyhow::{Result, anyhow};
use reqwest::Client;
use rmcp::model::CallToolResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    api,
    commands::post::ListArgs,
    models::{news::ZzkDocument, news::strip_html},
};

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct ListPostParams {
    /// 博客名称，API 中的 blog_app；不传则使用当前登录账号的博客名称。
    pub blog_app: Option<String>,
    /// 分页页码，从 1 开始。
    pub page_index: Option<u64>,
    /// 每页文章条数，默认 10。
    pub page_size: Option<u64>,
}

impl ListPostParams {
    pub fn into_list_args(self) -> ListArgs {
        ListArgs {
            name: self.blog_app,
            page_index: self.page_index.unwrap_or(1),
            page_size: self.page_size.unwrap_or(10),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PostSearchSort {
    Relevance,
    Time,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchPostParams {
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
    pub sort: Option<PostSearchSort>,
    /// 返回条数，范围 1..=15，默认 15。
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchPostQuery {
    #[serde(rename = "keyWords")]
    pub keywords: String,
    pub page_index: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_times_at_least: Option<u64>,
    #[serde(skip)]
    pub sort: PostSearchSort,
    #[serde(skip)]
    pub limit: u32,
}

impl SearchPostParams {
    pub fn into_search_query(self) -> Result<SearchPostQuery> {
        if self.keywords.trim().is_empty() {
            return Err(anyhow!("keywords 不能为空"));
        }

        let limit = self.limit.unwrap_or(15);
        if !(1..=15).contains(&limit) {
            return Err(anyhow!("limit 必须在 1..=15 之间"));
        }

        Ok(SearchPostQuery {
            keywords: self.keywords,
            page_index: self.page_index.unwrap_or(1),
            start_date: self.start_date,
            end_date: self.end_date,
            view_times_at_least: self.min_views,
            sort: self.sort.unwrap_or(PostSearchSort::Relevance),
            limit,
        })
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShowPostParams {
    /// 博客文章 ID。
    pub id: u64,
    /// 是否保留 HTML 原文；默认 false，只返回 MarkdownContent。
    pub include_html: Option<bool>,
}

impl ShowPostParams {
    pub fn validate(&self) -> Result<()> {
        if self.id == 0 {
            return Err(anyhow!("id 不能为 0"));
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PostDetail {
    id: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    content: String,
    markdown_content: String,
}

pub(crate) async fn list_posts_json(
    client: &Client,
    default_blog_app: &str,
    params: ListPostParams,
) -> Result<CallToolResult> {
    let mut args = params.into_list_args();
    let blog_app = match args.name.as_deref() {
        Some(name) if !name.trim().is_empty() => name.to_string(),
        _ if !default_blog_app.trim().is_empty() => default_blog_app.to_string(),
        _ => return Err(anyhow!("blog_app 不能为空；请提供 blog_app 或先登录")),
    };
    args.name = Some(blog_app.clone());

    let list = api::post::list_someone_post(client, &blog_app, args).await?;
    to_tool_items_result(&list)
}

pub(crate) async fn search_posts_json(
    client: &Client,
    params: SearchPostParams,
) -> Result<CallToolResult> {
    let query = params.into_search_query()?;
    let sort = query.sort;
    let limit = query.limit;
    let mut results = api::post::search_posts(client, query).await?;
    clean_search_results(&mut results);
    if matches!(sort, PostSearchSort::Time) {
        results.sort_by(|a, b| b.publish_time.cmp(&a.publish_time));
    }
    results.truncate(limit as usize);
    to_tool_items_result(&results)
}

pub(crate) async fn show_post_json(
    client: &Client,
    params: ShowPostParams,
) -> Result<CallToolResult> {
    params.validate()?;
    let include_html = params.include_html.unwrap_or(false);
    let content = api::post::get_post_body(client, params.id).await?;
    let markdown_content = tokio::task::spawn_blocking({
        let content = content.clone();
        move || html2md::parse_html(&content)
    })
    .await?;

    let detail = PostDetail {
        id: params.id,
        content: if include_html { content } else { String::new() },
        markdown_content,
    };
    to_tool_result(&detail)
}

fn clean_search_results(results: &mut [ZzkDocument]) {
    for doc in results {
        doc.title = strip_html(&doc.title);
        doc.content = strip_html(&doc.content);
    }
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
