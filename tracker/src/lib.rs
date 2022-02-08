use ksuid::Ksuid;
use miette::{miette, IntoDiagnostic, Result};
use scraper::{Html, Selector};
use serde::{Serialize, Serializer};
use sqlx::{
    database::{HasArguments, HasValueRef},
    encode::IsNull,
    mysql::MySqlTypeInfo,
    Database, Decode, Encode, MySql, Type,
};
use std::fmt;

pub async fn twitter() -> Result<u64> {
    let html: String =
        reqwest::get("https://twitter.com/chrisbiscardi")
            .await
            .into_diagnostic()?
            .text()
            .await
            .into_diagnostic()?;

    let document = Html::parse_document(&html);
    let selector = Selector::parse(
        r#"a[href="/chrisbiscardi/followers"]"#,
    )
    .unwrap();

    let num: u64 = document
        .select(&selector)
        .next()
        .ok_or(miette!("could not find followers element in twitter html document"))?
        .text()
        .find(|s| match s.chars().next() {
            Some(c) => ('0'..'9').contains(&c),
            None => false,
        })
        .ok_or(miette!("could not find followers number in followers element text"))?
        .chars()
        .filter(|c| c != &',')
        .collect::<String>()
        .parse()
        .into_diagnostic()?;

    Ok(num)
}

#[derive(Clone)]
pub struct Id(Ksuid);

impl Serialize for Id {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let id = self.0.to_base62();
        serializer.serialize_str(&id)
    }
}

impl fmt::Debug for Id {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_tuple("Id")
            .field(&self.0.to_base62())
            .finish()
    }
}

impl Id {
    pub fn new() -> Self {
        Id(Ksuid::generate())
    }
}

impl<'q> Encode<'q, MySql> for Id {
    fn encode_by_ref(
        &self,
        buf: &mut <MySql as HasArguments<'q>>::ArgumentBuffer,
    ) -> IsNull {
        let bytes: &[u8] = &self.0.to_base62().into_bytes();
        <&[u8] as Encode<MySql>>::encode(bytes, buf)
    }
}

impl Type<MySql> for Id {
    fn type_info() -> <MySql as Database>::TypeInfo {
        <&[u8] as Type<MySql>>::type_info()
    }
    fn compatible(ty: &MySqlTypeInfo) -> bool {
        <&[u8] as Type<MySql>>::compatible(ty)
    }
}

impl<'r> Decode<'r, MySql> for Id {
    fn decode(
        value: <MySql as HasValueRef<'r>>::ValueRef,
    ) -> Result<
        Id,
        Box<dyn std::error::Error + 'static + Send + Sync>,
    > {
        let value =
            <&[u8] as Decode<MySql>>::decode(value)?;
        let base62_ksuid = std::str::from_utf8(&value)?;
        Ok(Id(Ksuid::from_base62(base62_ksuid)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_twitter() {
        let res = twitter().await;
        assert_eq!(2 + 2, res.unwrap());
    }
}
