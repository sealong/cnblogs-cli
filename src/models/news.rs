use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::tools::timer::DateFormatExt;

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct NewsInfo {
    pub id: u64,
    pub title: String,
    pub summary: String,
    pub topic_id: u64,
    pub topic_icon: Option<String>,
    pub view_count: u64,
    pub comment_count: u64,
    pub digg_count: u64,
    #[serde(with = "crate::tools::timer::rfc3339_or_naive")]
    pub date_added: DateTime<Utc>,

    /// 派生字段：原文 URL。从 API 反序列化时为 None，序列化前由 logic 层填充
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl NewsInfo {
    /// 由 id 拼出博客园新闻原文 URL
    pub fn build_url(&self) -> String {
        format!("https://news.cnblogs.com/n/{}/", self.id)
    }
}

impl NewsInfo {
    pub fn into_format(self) -> String {
        format!(
            "{title}\n{summary}\n[#{id}][posted@ {date}][浏览：{vc}][评论：{cc}][点赞：{dc}]\n",
            title = self.title.bold().cyan(),
            summary = self.summary,
            id = self.id.bright_green(),
            date = self.date_added.as_time_age(),
            vc = self.view_count,
            cc = self.comment_count,
            dc = self.digg_count,
        )
    }

    pub fn into_title_format(self, index: usize) -> String {
        format!(
            "{index:>4}. {title}",
            index = index + 1,
            title = self.title
        ).bright_white().to_string()
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ZzkDocument {
    pub title: String,
    pub content: String,
    pub user_name: Option<String>,
    pub user_alias: Option<String>,
    pub publish_time: String,
    pub vote_times: i64,
    pub view_times: i64,
    pub comment_times: i64,
    pub uri: String,
    pub id: String,
}

lazy_static::lazy_static! {
    static ref HTML_TAG_REGEX: regex::Regex =
        regex::Regex::new(r"<[^>]*>").expect("无效的 HTML 标签正则");

    static ref NEWS_URI_ID_REGEX: regex::Regex =
        regex::Regex::new(r"/n/(\d+)").expect("无效的新闻 URI 正则");
}

fn strip_html(s: &str) -> String {
    let no_tags = HTML_TAG_REGEX.replace_all(s, "");
    decode_html_entities(&no_tags)
}

/// 解码常见 HTML 实体。&amp; 必须最后处理，避免把 &amp;lt; 错误展开成 <
fn decode_html_entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// 把博客园返回的发布时间渲染成相对时间。解析失败回退原始字符串。
/// 接口可能返回 "2026-05-18T10:23:45" 或 "2026-05-18T10:23:45.123" 之类，
/// 视作东八区本地时间转 UTC，再交给 DateFormatExt::as_time_age。
fn render_publish_time(raw: &str) -> String {
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return dt.with_timezone(&Utc).as_time_age();
    }
    for fmt in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(raw, fmt) {
            let utc = ndt - Duration::from_secs(8 * 3600);
            if let chrono::LocalResult::Single(dt) = Utc.from_local_datetime(&utc) {
                return dt.as_time_age();
            }
        }
    }
    raw.to_string()
}

impl ZzkDocument {
    /// 解析新闻数字 id：优先 id 字段，回退到从 uri 提取 `/n/<id>` 片段
    pub fn news_id(&self) -> Option<u64> {
        if let Ok(n) = self.id.parse::<u64>() {
            return Some(n);
        }
        NEWS_URI_ID_REGEX
            .captures(&self.uri)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<u64>().ok())
    }

    pub fn into_title_format(self, index: usize) -> String {
        let title = strip_html(&self.title);
        format!("{index:>4}. {title}", index = index + 1, title = title)
            .bright_white()
            .to_string()
    }

    pub fn into_format(self, index: usize) -> String {
        let title = strip_html(&self.title);
        let summary = strip_html(&self.content);
        let summary = if summary.chars().count() > 120 {
            format!("{}...", summary.chars().take(120).collect::<String>())
        } else {
            summary
        };
        let id_display = match self.news_id() {
            Some(n) => format!("#{}", n.bright_green()),
            None => format!("#{}", self.id.bright_green()),
        };
        // 博客园 ZzkDocuments 对新闻类目的 UserName/UserAlias 永远为 null，
        // 该段为空时整段省略，避免输出"未知"这种无信息噪声
        let author_segment = match self.user_name.as_deref().filter(|s| !s.is_empty()) {
            Some(name) => format!("{}  |  ", name.yellow()),
            None => String::new(),
        };
        format!(
            "{index:>4}. {title}\n     {summary}\n     [{id}] {author}浏览: {views}  |  推荐: {votes}  |  评论: {comments}  |  发布于: {time}\n     {uri}\n",
            index = index + 1,
            title = title.bold().cyan(),
            summary = summary.dimmed(),
            id = id_display,
            author = author_segment,
            views = self.view_times,
            votes = self.vote_times,
            comments = self.comment_times,
            time = render_publish_time(&self.publish_time),
            uri = self.uri.dimmed(),
        )
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct NewsDetail {
    pub news_id: u64,
    #[serde(default, deserialize_with = "deserialize_tags")]
    pub tags: Option<Vec<String>>,
    pub author: String,
    pub title: String,
    pub publish_time: String,
    pub pic_name: Option<String>,
    pub content: String,

    /// 派生字段：把 `content`（HTML）转成 Markdown。
    /// 反序列化时为 None；JSON 输出路径会通过 spawn_blocking 填充，避免 skill 自己跑 html2md
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub markdown_content: Option<String>,
}

fn deserialize_tags<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Tags {
        Array(Vec<String>),
        String(String),
        Null,
    }

    match Tags::deserialize(deserializer)? {
        Tags::Array(v) => Ok(Some(v)),
        Tags::String(s) => Ok(Some(vec![s])),
        Tags::Null => Ok(None),
    }
}

impl NewsDetail {
    pub fn into_header_format(&self) -> String {
        let separator = "=".repeat(80);
        format!(
            "{separator}\n{title}\n作者: {author}  |  发布时间: {time}  |  ID: #{id}\n{separator}\n",
            title = self.title.bold().bright_cyan(),
            author = self.author.yellow(),
            time = self.publish_time,
            id = self.news_id.bright_green(),
        )
    }

    pub fn clean_content(&self) -> String {
        self.content.replace("\r\n", "\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let a = r#"{
                "Id": 813264,
                "Title": "马斯克兑现承诺，开源X推荐算法！100% AI驱动，0人工规则",
                "Summary": "新智元报道 编辑：定慧 马斯克兑现承诺，X平台全新推荐算法正式开源！这套由 Grok 驱动的 AI 系统，完全取代了人工规则，通过 15 种行为预测精准计算每条帖子的命运。 1 月 11 日，马斯克在X平台上发了一条帖子，宣布将在 7 天内开源X平台全新的推荐算法。 他还承诺，此后每 4 周重复一次",
                "TopicId": 570,
                "TopicIcon": "https://images0.cnblogs.com/news_topic/20150702090016187.png",
                "ViewCount": 37,
                "CommentCount": 0,
                "DiggCount": 2,
                "DateAdded": "2026-01-20T15:40:00+08:00"
            }"#;

        let aa: NewsInfo = serde_json::from_str(a).unwrap();
        assert_eq!(aa.id, 813264);
    }

    #[test]
    fn news_id_from_numeric_id_field() {
        let d = ZzkDocument {
            id: "813264".into(),
            uri: "https://news.cnblogs.com/n/813264/".into(),
            ..Default::default()
        };
        assert_eq!(d.news_id(), Some(813264));
    }

    #[test]
    fn news_id_falls_back_to_uri() {
        let d = ZzkDocument {
            id: "doc-abc-xyz".into(),
            uri: "https://news.cnblogs.com/n/812345/".into(),
            ..Default::default()
        };
        assert_eq!(d.news_id(), Some(812345));
    }

    #[test]
    fn decode_entities_handles_amp_last() {
        // 关键：&amp;lt; 应解码为 "&lt;"，而不是 "<"
        assert_eq!(decode_html_entities("&amp;lt;"), "&lt;");
        assert_eq!(decode_html_entities("a&nbsp;b&amp;c&lt;d&gt;e&quot;f&#39;g"), "a b&c<d>e\"f'g");
    }

    #[test]
    fn strip_html_decodes_entities() {
        let raw = "<p>Tom&nbsp;&amp;&nbsp;Jerry &lt;hi&gt;</p>";
        assert_eq!(strip_html(raw), "Tom & Jerry <hi>");
    }

    #[test]
    fn render_publish_time_falls_back_on_unparseable() {
        assert_eq!(render_publish_time("not-a-date"), "not-a-date");
    }

    #[test]
    fn render_publish_time_parses_naive_format() {
        // 应能解析成功并返回带方括号的时间格式（绝对或相对）
        let out = render_publish_time("2026-05-18T10:23:45");
        assert!(out.starts_with('['), "got: {}", out);
    }

    #[test]
    fn news_id_none_when_neither_matches() {
        let d = ZzkDocument {
            id: "doc-abc".into(),
            uri: "https://example.com/whatever".into(),
            ..Default::default()
        };
        assert_eq!(d.news_id(), None);
    }
}
