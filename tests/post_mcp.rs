use cnblogs_lib::mcp::post::{ListPostParams, PostSearchSort, SearchPostParams, ShowPostParams};

#[test]
fn list_params_use_cli_defaults_and_optional_blog_app() {
    let args = ListPostParams::default().into_list_args();

    assert_eq!(args.name, None);
    assert_eq!(args.page_index, 1);
    assert_eq!(args.page_size, 10);

    let args = ListPostParams {
        blog_app: Some("cnblogs".to_string()),
        page_index: Some(2),
        page_size: Some(5),
    }
    .into_list_args();

    assert_eq!(args.name.as_deref(), Some("cnblogs"));
    assert_eq!(args.page_index, 2);
    assert_eq!(args.page_size, 5);
}

#[test]
fn search_params_map_to_blog_post_search_query() {
    let query = SearchPostParams {
        keywords: "rust".to_string(),
        page_index: Some(2),
        start_date: Some("2026-06-01".to_string()),
        end_date: Some("2026-06-04".to_string()),
        min_views: Some(100),
        sort: Some(PostSearchSort::Time),
        limit: Some(5),
    }
    .into_search_query()
    .expect("valid search params");

    assert_eq!(query.keywords, "rust");
    assert_eq!(query.page_index, 2);
    assert_eq!(query.start_date.as_deref(), Some("2026-06-01"));
    assert_eq!(query.end_date.as_deref(), Some("2026-06-04"));
    assert_eq!(query.view_times_at_least, Some(100));
    assert_eq!(query.sort, PostSearchSort::Time);
    assert_eq!(query.limit, 5);
}

#[test]
fn search_params_reject_empty_keywords() {
    let err = SearchPostParams {
        keywords: "  ".to_string(),
        page_index: None,
        start_date: None,
        end_date: None,
        min_views: None,
        sort: None,
        limit: None,
    }
    .into_search_query()
    .expect_err("empty keywords should fail");

    assert!(err.to_string().contains("keywords"));
}

#[test]
fn search_params_reject_limit_outside_api_page_size() {
    let err = SearchPostParams {
        keywords: "rust".to_string(),
        page_index: None,
        start_date: None,
        end_date: None,
        min_views: None,
        sort: None,
        limit: Some(16),
    }
    .into_search_query()
    .expect_err("limit > 15 should fail");

    assert!(err.to_string().contains("limit"));
}

#[test]
fn show_params_default_to_markdown_only() {
    let params = ShowPostParams {
        id: 19276632,
        include_html: None,
    };

    assert_eq!(params.id, 19276632);
    assert!(!params.include_html.unwrap_or(false));
    assert!(params.validate().is_ok());
}

#[test]
fn show_params_reject_zero_id() {
    let err = ShowPostParams {
        id: 0,
        include_html: None,
    }
    .validate()
    .expect_err("zero id should fail");

    assert!(err.to_string().contains("id"));
}
