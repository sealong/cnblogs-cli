use anyhow::Result;
use reqwest::{Client, Response};
use serde::Serialize;

use crate::{api::urls::OPENAPI, models::fav::FavInfo, tools::IntoAnyhowResult};

pub async fn list_bookmarks(
    c: &Client,
    page: impl Serialize + Send + Sync,
) -> Result<Vec<FavInfo>> {
    let resp = raw_list_bookmarks(c, page).await?;
    resp.error_for_status()?
        .json()
        .await
        .into_anyhow_result()
}

pub async fn raw_list_bookmarks(
    c: &Client,
    page: impl Serialize + Send + Sync,
) -> Result<Response> {
    let url = format!("{}/{}", OPENAPI, "bookmarks");
    c.get(url).query(&page).send().await.into_anyhow_result()
}
