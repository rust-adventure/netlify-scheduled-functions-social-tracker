use std::env;

use aws_lambda_events::{
    encodings::Body,
    event::apigw::{
        ApiGatewayProxyRequest, ApiGatewayProxyResponse,
    },
};
use http::HeaderMap;
use lambda_runtime::{handler_fn, Context, Error};
use once_cell::sync::OnceCell;
use sqlx::{mysql::MySqlPoolOptions, MySql, Pool};
use tracker::{twitter, Id};

static POOL: OnceCell<Pool<MySql>> = OnceCell::new();

#[tokio::main]
async fn main() -> Result<(), Error> {
    let database_url = env::var("DATABASE_URL").unwrap();
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    POOL.get_or_init(|| pool);

    let handler_fn = handler_fn(handler);
    lambda_runtime::run(handler_fn).await?;
    Ok(())
}

async fn handler(
    _: ApiGatewayProxyRequest,
    _: Context,
) -> Result<ApiGatewayProxyResponse, Error> {
    let count_res = twitter().await;
    match count_res {
        Ok(count) => {
            sqlx::query!(
                r#"
        INSERT INTO tracking (
            id, recorded_value
        ) VALUES (?, ?)"#,
                Id::new(),
                count,
            )
            .execute(POOL.get().unwrap())
            .await?;
            Ok(ApiGatewayProxyResponse {
                status_code: 200,
                headers: HeaderMap::new(),
                multi_value_headers: HeaderMap::new(),
                body: Some(Body::Text(format!(
                    "Hello, {}!",
                    count
                ))),
                is_base64_encoded: Some(false),
            })
        }
        Err(_err) => Ok(ApiGatewayProxyResponse {
            status_code: 200,
            headers: HeaderMap::new(),
            multi_value_headers: HeaderMap::new(),
            body: Some(Body::Text(format!("error",))),
            is_base64_encoded: Some(false),
        }),
    }
}
