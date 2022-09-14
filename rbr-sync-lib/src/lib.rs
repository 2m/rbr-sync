use futures::future::try_join_all;
use reqwest::{
    header::{self, InvalidHeaderValue},
    Url,
};
use serde::Deserialize;
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
}

mod database {
    use reqwest::{Client, Url};
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct PropertyResult {
        pub id: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct Response {
        pub results: Vec<PropertyResult>,
    }

    pub async fn query(id: &str, client: &Client, url: &Url) -> Result<Response, crate::AppError> {
        let resp = client
            .post(url.join(format!("databases/{id}/query").as_str())?)
            .send()
            .await?
            .json::<Response>()
            .await?;
        Ok(resp)
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
            .await?
            .json::<T>()
            .await?;
        Ok(resp)
    }
}

#[derive(Debug, Deserialize)]
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

    let db = database::query(db_id, &client, &url).await?;
    let stages_future = db.results.iter().map(|result| async {
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
