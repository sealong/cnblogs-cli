use clap::{Args, Subcommand};
use serde::Serialize;

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
}

/// 展示新闻内容
#[derive(Debug, Args, Serialize)]
pub struct ShowArgs {
    /// 新闻ID，必传
    #[serde(skip)]
    #[clap(value_parser = validate_non_zero_id, required = true)]
    pub id: u64,
}

/// 搜索新闻
#[derive(Debug, Args, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchArgs {
    /// 搜索关键字
    #[serde(rename = "keyWords")]
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
}
