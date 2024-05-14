use rss::Channel;
use snafu::{ResultExt, Snafu};

#[derive(Debug, Clone)]
pub struct Subscription {
    pub bt_url: String,
    pub anime_title: String,
    pub file_name: String,
}

// Fetch feed from the mikanani.me rss feed
pub async fn get_feed(url: impl Into<String>) -> Result<Vec<Subscription>, Error> {
    let content = reqwest::get(url.into())
        .await
        .context(FetchFeedSnafu)?
        .bytes()
        .await
        .context(FetchFeedSnafu)?;
    let channel = Channel::read_from(&content[..]).context(ReadFeedSnafu)?;

    let mut subscriptions = Vec::new();
    for item in channel.items {
        let subscription = convert(&item).await?;
        subscriptions.push(subscription);
    }

    Ok(subscriptions)
}

async fn get_anime_name_from_episode_page(url: &str) -> Result<String, Error> {
    let content = reqwest::get(url)
        .await
        .context(FetchEpisodePageSnafu)?
        .text()
        .await
        .context(FetchEpisodePageSnafu)?;
    let document = scraper::Html::parse_document(&content);

    let title = document
        .select(&scraper::Selector::parse("p[class='bangumi-title']").unwrap())
        .next()
        .ok_or(Error::ParseEpisodePage {
            url: url.to_owned(),
        })?
        .select(&scraper::Selector::parse("a[class='w-other-c']").unwrap())
        .next()
        .ok_or(Error::ParseEpisodePage {
            url: url.to_owned(),
        })?
        .text()
        .collect::<String>();

    Ok(title)
}

async fn convert(item: &rss::Item) -> Result<Subscription, Error> {
    let link = item.link.as_ref().ok_or(Error::ConvertFeed {
        item: item.clone(),
        entity: "link".into(),
    })?;
    let name = item.title.as_ref().ok_or(Error::ConvertFeed {
        item: item.clone(),
        entity: "title".into(),
    })?;
    let bt_url = item
        .enclosure
        .as_ref()
        .ok_or(Error::ConvertFeed {
            item: item.clone(),
            entity: "enclosure".into(),
        })?
        .url
        .clone();

    let title = get_anime_name_from_episode_page(link).await?;

    Ok(Subscription {
        bt_url,
        anime_title: title,
        file_name: name.clone(),
    })
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("Failed to fetch feed"))]
    FetchFeed { source: reqwest::Error },

    #[snafu(display("Failed to read feed"))]
    ReadFeed { source: rss::Error },

    #[snafu(display("Failed to convert {item:?} with empty {entity}"))]
    ConvertFeed { item: rss::Item, entity: String },

    #[snafu(display("Failed to fetch episode page"))]
    FetchEpisodePage { source: reqwest::Error },

    #[snafu(display("Failed to parse episode page"))]
    ParseEpisodePage { url: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_feed() {
        let sub = get_feed("https://mikanani.me/RSS/MyBangumi?token=7eyqLval7M7aHnZ08QNtMAVbg%2fipV4sY5pAYcasKRBI%3d")
            .await
            .expect("Failed to fetch feed");
        dbg!(sub);
    }

    #[tokio::test]
    async fn test_convert() {
        let title = get_anime_name_from_episode_page(
            "https://mikanani.me/Home/Episode/e6057aa20463920c5b7518aa40c8a3d284f10e56",
        )
        .await
        .expect("Failed to fetch episode page");

        assert_eq!(title, "无职转生Ⅱ ～到了异世界就拿出真本事～ 第2部分");
    }
}
