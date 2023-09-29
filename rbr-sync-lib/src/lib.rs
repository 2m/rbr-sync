use futures::future::try_join_all;
use reqwest::{
    header::{self, InvalidHeaderValue},
    Response, Url,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;
use url::ParseError;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Wrong URL provided")]
    WrongUrl(#[from] ParseError),

    #[error("Error while sending HTTP request")]
    HttpError(#[from] reqwest::Error),

    #[error("Invalid authorization token")]
    WrongToken(#[from] InvalidHeaderValue),

    #[error("Received error from server. Status: {0}, body: {0}")]
    WrongResponseCode(u16, String),

    #[error("Unable to deserialize server response")]
    DeserizalizationError(#[from] serde_json::Error),
}

mod database {
    use std::collections::HashMap;

    use reqwest::{Client, Url};
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct PropertyResult {
        pub id: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct Response {
        pub results: Vec<PropertyResult>,
        pub has_more: bool,
        pub next_cursor: Option<String>,
    }

    pub async fn query(
        id: &str,
        client: &Client,
        url: &Url,
    ) -> Result<Vec<PropertyResult>, crate::AppError> {
        let mut results = Vec::new();
        let mut has_more = true;
        let mut start_cursor: Option<String> = None;

        while has_more {
            let mut body = HashMap::new();
            if start_cursor.clone().is_some() {
                body.insert("start_cursor", start_cursor.clone().unwrap());
            }

            let resp = client
                .post(url.join(format!("databases/{id}/query").as_str())?)
                .json(&body)
                .send()
                .await?;

            let response = crate::deserialize_successful_response::<Response>(resp).await?;

            results.extend(response.results);
            has_more = response.has_more;
            start_cursor = response.next_cursor;
        }

        Ok(results)
    }
}

mod page {
    use reqwest::{Client, Url};
    use serde::{de::DeserializeOwned, Deserialize};

    #[derive(Debug, Deserialize)]
    pub struct Number {
        pub number: i32,
    }

    #[derive(Debug, Deserialize)]
    pub struct Title {
        pub results: Vec<TitleResult>,
    }

    #[derive(Debug, Deserialize)]
    pub struct TitleResult {
        pub title: Text,
    }

    #[derive(Debug, Deserialize)]
    pub struct Text {
        pub plain_text: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct MultiSelect {
        pub multi_select: Vec<Select>,
    }

    #[derive(Debug, Deserialize)]
    pub struct Select {
        pub name: String,
    }

    pub async fn property<T: DeserializeOwned>(
        id: &str,
        page: &str,
        client: &Client,
        url: &Url,
    ) -> Result<T, crate::AppError> {
        let resp = client
            .get(url.join(format!("pages/{page}/properties/{id}").as_str())?)
            .send()
            .await?;

        return crate::deserialize_successful_response(resp).await;
    }
}

async fn deserialize_successful_response<T: DeserializeOwned>(
    resp: Response,
) -> Result<T, crate::AppError> {
    let status = resp.status();
    let text = resp.text().await?;

    if status.is_success() {
        return serde_json::from_str::<T>(&text).map_err(crate::AppError::DeserizalizationError);
    } else {
        return Err(crate::AppError::WrongResponseCode(status.into(), text));
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Stage {
    pub id: i32,
    pub title: String,
    pub tags: Vec<String>,
}

pub async fn stages(token: &str, db_id: &str) -> Result<Vec<Stage>, AppError> {
    let url = Url::parse("https://api.notion.com/v1/")?;

    let mut headers = header::HeaderMap::new();
    headers.insert(
        "Notion-Version",
        header::HeaderValue::from_static("2022-06-28"),
    );
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(format!("Bearer {token}").as_str())?,
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let results = database::query(db_id, &client, &url).await?;
    let stages_future = results.iter().map(|result| async {
        let id = page::property::<page::Number>("ID", result.id.as_str(), &client, &url);
        let name = page::property::<page::Title>("Name", result.id.as_str(), &client, &url);
        let tags = page::property::<page::MultiSelect>("Tags", result.id.as_str(), &client, &url);

        match (id.await, name.await, tags.await) {
            (Ok(id), Ok(name), Ok(tags)) => Ok(Stage {
                id: id.number,
                title: name.results.first().unwrap().title.plain_text.clone(),
                tags: tags.multi_select.iter().map(|t| t.name.clone()).collect(),
            }),
            (Err(err), _, _) => Err(err),
            (_, Err(err), _) => Err(err),
            (_, _, Err(err)) => Err(err),
        }
    });
    try_join_all(stages_future).await
}
