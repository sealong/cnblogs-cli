use clap::{Args, Subcommand, ValueEnum, builder::NonEmptyStringValueParser};
use serde::Serialize;

/// search 客户端排序模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SearchSort {
    /// 按 cnblogs 搜索引擎相关度（默认，API 原序）
    Relevance,
    /// 按 PublishTime 倒序（客户端排序）
    Time,
}

use crate::commands::validate_non_zero_id;

#[derive(Debug, Args)]
pub struct NewsCommand {
    #[clap(subcommand)]
    pub commands: NewsAction,
}

#[derive(Debug, Subcommand)]
pub enum NewsAction {
    List(ListArgs),
    Show(ShowArgs),
    Search(SearchArgs),
    /// 热门新闻（指定时间区间）
    Hot(HotArgs),
    /// 本周热门新闻
    #[command(name = "hot-week")]
    HotWeek(HotWeekArgs),
    /// 推荐新闻
    Recommended(RecommendedArgs),
}

#[derive(Debug, Args, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListArgs {
    /// 分页页码（从1开始）
    #[arg(long = "page-index", default_value_t = 1)]
    pub page_index: u64,

    /// 每页显示的条数，默认10
    #[arg(long = "page-size", default_value_t = 10)]
    pub page_size: u64,

    /// 仅输出新闻标题
    #[arg(long = "title-only", default_value_t = false)]
    #[serde(skip)]
    pub title_only: bool,

    /// 仅输出新闻 ID（每行一个），便于管道：`cnb news list --ids-only | cnb news show --stdin`
    #[arg(long = "ids-only", default_value_t = false)]
    #[serde(skip)]
    pub ids_only: bool,
}

/// 展示新闻内容
#[derive(Debug, Args)]
pub struct ShowArgs {
    /// 新闻 ID，可传一个或多个（空格分隔）；与 --stdin 互斥
    #[clap(
        value_parser = validate_non_zero_id,
        num_args = 0..,
        required_unless_present = "stdin",
        conflicts_with = "stdin",
    )]
    pub ids: Vec<u64>,

    /// 从标准输入读取 ID 列表（空格或换行分隔），适合脚本 / skill 批量喂入
    #[arg(long)]
    pub stdin: bool,

    /// 批量请求的并发上限，默认 8；超过这个数的 ID 会排队等待 permit
    #[arg(
        long = "max-concurrent",
        default_value_t = 8,
        value_parser = clap::value_parser!(u32).range(1..=64),
    )]
    pub max_concurrent: u32,

    /// 批量模式默认 omit Content（HTML 原文），只输出 MarkdownContent。
    /// 加此 flag 时连 HTML 一起输出；单 ID 模式不受影响（始终保留 HTML）
    #[arg(long = "include-html", default_value_t = false)]
    pub include_html: bool,
}

/// 搜索新闻
#[derive(Debug, Args, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchArgs {
    /// 搜索关键字
    #[serde(rename = "keyWords")]
    #[clap(value_parser = NonEmptyStringValueParser::new(), required = true)]
    pub keywords: String,

    /// 分页页码（从1开始）
    #[arg(long = "page-index", default_value_t = 1)]
    pub page_index: u64,

    /// 开始日期 (格式: YYYY-MM-DD)
    #[arg(long = "start-date")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,

    /// 结束日期 (格式: YYYY-MM-DD)
    #[arg(long = "end-date")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,

    /// 最低浏览次数
    #[arg(long = "min-views")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_times_at_least: Option<u64>,

    /// 仅输出新闻标题
    #[arg(long = "title-only", default_value_t = false)]
    #[serde(skip)]
    pub title_only: bool,

    /// 仅输出新闻 ID（每行一个），便于管道：`cnb news search "AI" --ids-only | cnb news show --stdin`
    #[arg(long = "ids-only", default_value_t = false)]
    #[serde(skip)]
    pub ids_only: bool,

    /// 客户端排序方式（cnblogs 搜索 API 默认按相关度返回；time 时本地按 PublishTime 倒序）
    #[arg(long = "sort", default_value = "relevance", value_enum)]
    #[serde(skip)]
    pub sort: SearchSort,

    /// 限制返回条数（cnblogs 搜索单页最多 15 条，默认 15）
    #[arg(
        long = "limit",
        default_value_t = 15,
        value_parser = clap::value_parser!(u32).range(1..=15),
    )]
    #[serde(skip)]
    pub limit: u32,
}

/// 热门新闻（区间）
#[derive(Debug, Args, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotArgs {
    /// 开始日期 (格式: YYYY-MM-DD)。与 --end-date 必须成对出现，且与 --last-days 互斥
    #[arg(
        long = "start-date",
        conflicts_with = "last_days",
        requires = "end_date"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,

    /// 结束日期 (格式: YYYY-MM-DD)
    #[arg(
        long = "end-date",
        conflicts_with = "last_days",
        requires = "start_date"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,

    /// 取最近 N 天（end-date = 今天，start-date = 今天-N）；不传任何日期参数时默认 7
    #[arg(long = "last-days")]
    #[serde(skip)]
    pub last_days: Option<u32>,

    /// 分页页码（从1开始）
    #[arg(long = "page-index", default_value_t = 1)]
    pub page_index: u64,

    /// 每页显示的条数，默认10
    #[arg(long = "page-size", default_value_t = 10)]
    pub page_size: u64,

    /// 仅输出新闻标题
    #[arg(long = "title-only", default_value_t = false)]
    #[serde(skip)]
    pub title_only: bool,

    /// 仅输出新闻 ID（每行一个），便于管道
    #[arg(long = "ids-only", default_value_t = false)]
    #[serde(skip)]
    pub ids_only: bool,
}

/// 本周热门新闻
#[derive(Debug, Args, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotWeekArgs {
    /// 分页页码（从1开始）
    #[arg(long = "page-index", default_value_t = 1)]
    pub page_index: u64,

    /// 每页显示的条数，默认10
    #[arg(long = "page-size", default_value_t = 10)]
    pub page_size: u64,

    /// 仅输出新闻标题
    #[arg(long = "title-only", default_value_t = false)]
    #[serde(skip)]
    pub title_only: bool,

    /// 仅输出新闻 ID（每行一个），便于管道
    #[arg(long = "ids-only", default_value_t = false)]
    #[serde(skip)]
    pub ids_only: bool,
}

/// 推荐新闻
#[derive(Debug, Args, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedArgs {
    /// 分页页码（从1开始）
    #[arg(long = "page-index", default_value_t = 1)]
    pub page_index: u64,

    /// 每页显示的条数，默认10
    #[arg(long = "page-size", default_value_t = 10)]
    pub page_size: u64,

    /// 仅输出新闻标题
    #[arg(long = "title-only", default_value_t = false)]
    #[serde(skip)]
    pub title_only: bool,

    /// 仅输出新闻 ID（每行一个），便于管道
    #[arg(long = "ids-only", default_value_t = false)]
    #[serde(skip)]
    pub ids_only: bool,
}
