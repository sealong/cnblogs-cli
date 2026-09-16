use cnblogs_lib::mcp::news::{HotNewsParams, NewsSearchSort, SearchNewsParams, ShowNewsParams};

#[test]
fn list_params_use_cli_defaults() {
    let args = cnblogs_lib::mcp::news::ListNewsParams::default().into_list_args();

    assert_eq!(args.page_index, 1);
    assert_eq!(args.page_size, 10);
    assert!(!args.title_only);
    assert!(!args.ids_only);
}

#[test]
fn search_params_map_to_news_search_args() {
    let args = SearchNewsParams {
        keywords: "rust".to_string(),
        page_index: Some(2),
        start_date: Some("2026-06-01".to_string()),
        end_date: Some("2026-06-04".to_string()),
        min_views: Some(100),
        sort: Some(NewsSearchSort::Time),
        limit: Some(5),
    }
    .into_search_args()
    .expect("valid search args");

    assert_eq!(args.keywords, "rust");
    assert_eq!(args.page_index, 2);
    assert_eq!(args.start_date.as_deref(), Some("2026-06-01"));
    assert_eq!(args.end_date.as_deref(), Some("2026-06-04"));
    assert_eq!(args.view_times_at_least, Some(100));
    assert_eq!(args.sort, cnblogs_lib::commands::news::SearchSort::Time);
    assert_eq!(args.limit, 5);
}

#[test]
fn hot_params_reject_mixed_relative_and_absolute_dates() {
    let err = HotNewsParams {
        last_days: Some(7),
        start_date: Some("2026-06-01".to_string()),
        end_date: Some("2026-06-04".to_string()),
        page_index: None,
        page_size: None,
    }
    .into_hot_args()
    .expect_err("mixed date modes should fail");

    assert!(err.to_string().contains("last_days"));
}

#[test]
fn show_params_default_to_markdown_only_batch_output() {
    let args = ShowNewsParams {
        ids: Some(vec![822790, 822789]),
        title: None,
        max_concurrent: None,
        include_html: None,
    }
    .into_show_args()
    .expect("valid show args");

    assert_eq!(args.ids, vec![822790, 822789]);
    assert!(!args.stdin);
    assert_eq!(args.max_concurrent, 8);
    assert!(!args.include_html);
}

#[test]
fn show_params_accept_title_without_ids() {
    let params = ShowNewsParams {
        ids: None,
        title: Some("癌症之王治疗重大突破，全体掌声起立！".to_string()),
        max_concurrent: None,
        include_html: None,
    };

    assert_eq!(
        params.normalized_title().as_deref(),
        Some("癌症之王治疗重大突破，全体掌声起立！")
    );
    assert!(params.into_show_args().is_err());
}

#[test]
fn show_params_reject_empty_title_and_missing_ids() {
    let err = ShowNewsParams {
        ids: None,
        title: Some("  ".to_string()),
        max_concurrent: None,
        include_html: None,
    }
    .validate_lookup()
    .expect_err("empty title should fail");

    assert!(err.to_string().contains("ids 或 title"));
}
