use chrono::NaiveDate;
use once_cell::sync::Lazy;
use rss::Channel;
use snafu::{ResultExt, Snafu};
use url::Url;

use crate::store;

static MIKANANI_DOMAIN: Lazy<String> =
    Lazy::new(|| std::env::var("MIKANANI_DOMAIN").unwrap_or_else(|_| "mikanani.me".to_owned()));

#[derive(Debug, Clone)]
pub struct Subscription {
    pub magnet: String,
    pub anime: Anime,
    pub name: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Anime {
    pub rss: String,
    pub weekday: String,
    pub name: String,
    pub air_date: NaiveDate,
    pub bangumi_link: String,
}

// Fetch feed from the mikanani.me rss feed
pub async fn get_feed(url: &str) -> Result<Vec<Subscription>, Error> {
    let u = generate_url(url)?;

    let content = reqwest::get(u.to_string())
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

async fn get_info_from_episode_page(url: &str) -> Result<(String, Anime), Error> {
    let u = generate_url(url)?;

    let content = reqwest::get(u.to_string())
        .await
        .with_context(|_| FetchEpisodePageSnafu {
            url: url.to_owned(),
        })?
        .text()
        .await
        .with_context(|_| FetchEpisodePageSnafu {
            url: url.to_owned(),
        })?;
    let document = scraper::Html::parse_document(&content);

    let anime_url = document
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
        .value()
        .attr("href")
        .unwrap_or_default()
        .to_owned();

    let magnet = document
        .select(&scraper::Selector::parse("div[class='leftbar-nav']").unwrap())
        .next()
        .ok_or(Error::ParseEpisodePage {
            url: url.to_owned(),
        })?
        .select(&scraper::Selector::parse("a[class='btn episode-btn']").unwrap())
        .find(|element| {
            element
                .value()
                .attr("href")
                .map(|href| href.starts_with("magnet:?"))
                .unwrap_or(false)
        })
        .map(|element| element.value().attr("href").unwrap().to_owned())
        .ok_or(Error::ParseEpisodePage {
            url: url.to_owned(),
        })?;

    let anime = get_info_from_anime_page(&anime_url).await?;

    Ok((magnet, anime))
}

async fn get_info_from_anime_page(url: &str) -> Result<Anime, Error> {
    // 从url中解析出bangumi_id和subgroup_id
    let (bangumi_id, subgroup_id) = parse_url(url)?;

    // 如果数据库中已经有该剧集的信息，则直接返回
    if let Some(anime) = store::Db::get_anime()
        .and_then(|db| db.get(&bangumi_id))
        .context(LinkDatabaseSnafu)?
    {
        return Ok(anime);
    }

    let u = generate_url(url)?;

    let content = reqwest::get(u.to_string())
        .await
        .with_context(|_| FetchEpisodePageSnafu {
            url: url.to_owned(),
        })?
        .text()
        .await
        .with_context(|_| FetchEpisodePageSnafu {
            url: url.to_owned(),
        })?;
    let document = scraper::Html::parse_document(&content);

    // 该剧集该字幕组的rss链接
    let rss = format!(
        "https://{}/RSS/Bangumi?bangumiId={}&subgroupid={}",
        MIKANANI_DOMAIN.to_string(),
        bangumi_id,
        subgroup_id
    );
    let element = document
        .select(&scraper::Selector::parse("div[class='pull-left leftbar-container']").unwrap())
        .next()
        .ok_or(Error::ParseAnimePage {
            url: url.to_owned(),
            error: "rss".into(),
        })?;

    // 该剧集是周几更新
    let weekday = element
        .select(&scraper::Selector::parse("p[class='bangumi-info']").unwrap())
        .find_map(|element| {
            let text = element.text().collect::<String>();
            if let Some(weekday) = text.strip_prefix("放送日期：") {
                Some(weekday.to_owned())
            } else {
                None
            }
        })
        .ok_or(Error::ParseAnimePage {
            url: url.to_owned(),
            error: "weekday".into(),
        })?;

    // 该剧集的名字
    let name = element
        .select(&scraper::Selector::parse("p[class='bangumi-title']").unwrap())
        .next()
        .ok_or(Error::ParseAnimePage {
            url: url.to_owned(),
            error: "name".into(),
        })?
        .text()
        .collect::<String>();

    // 该剧集的首播日期
    let air_date = element
        .select(&scraper::Selector::parse("p[class='bangumi-info']").unwrap())
        .find_map(|element| {
            let text = element.text().collect::<String>();
            if let Some(air_date) = text.strip_prefix("放送开始：") {
                Some(air_date.to_owned())
            } else {
                None
            }
        })
        .ok_or(Error::ParseAnimePage {
            url: url.to_owned(),
            error: "air_date".into(),
        })?;
    let air_date =
        NaiveDate::parse_from_str(&air_date, "%m/%d/%Y").map_err(|_| Error::ParseAnimePage {
            url: url.to_owned(),
            error: format!("air_date: {}", air_date),
        })?;

    // 对应的Bangumi番组计划链接
    let bangumi_link = element
        .select(&scraper::Selector::parse("p[class='bangumi-info']").unwrap())
        .find(|element| {
            let text = element.text().collect::<String>();
            text.contains("Bangumi番组计划链接：")
        })
        .map(|element| {
            element
                .select(&scraper::Selector::parse("a[class='w-other-c']").unwrap())
                .next()
                .map(|element| element.value().attr("href").to_owned())
        })
        .flatten()
        .flatten()
        .ok_or(Error::ParseAnimePage {
            url: url.to_owned(),
            error: "bangumi_link".into(),
        })?
        .to_owned();

    let anime = Anime {
        rss,
        weekday,
        name,
        air_date,
        bangumi_link,
    };

    store::Db::get_anime()
        .and_then(|db| db.insert(&bangumi_id, anime.clone()))
        .context(LinkDatabaseSnafu)?;

    Ok(anime)
}

async fn convert(item: &rss::Item) -> Result<Subscription, Error> {
    let link = item.link.as_ref().ok_or(Error::ConvertFeed {
        item: item.clone(),
        entity: "link".into(),
    })?;

    let name = item
        .title
        .as_ref()
        .ok_or(Error::ConvertFeed {
            item: item.clone(),
            entity: "title".into(),
        })?
        .to_string();

    let (magnet, title) = get_info_from_episode_page(link).await?;

    Ok(Subscription {
        magnet,
        name,
        anime: title,
    })
}

/// 从url中解析出bangumi_id和subgroup_id
fn parse_url(url: &str) -> Result<(String, String), Error> {
    let u = generate_url(url)?;

    let bangumi_id = u
        .path_segments()
        .ok_or(Error::ParseUrl {
            url: url.to_owned(),
        })?
        .last()
        .ok_or(Error::ParseUrl {
            url: url.to_owned(),
        })?;

    let subgroup_id = u.fragment().ok_or(Error::ParseUrl {
        url: url.to_owned(),
    })?;

    if bangumi_id.is_empty() || subgroup_id.is_empty() {
        return Err(Error::ParseUrl {
            url: url.to_owned(),
        });
    }

    Ok((bangumi_id.to_owned(), subgroup_id.to_owned()))
}

/// 生成指定子域名的url
/// 该函数会将url的scheme和host都设置为https和MIKANANI_DOMAIN环境变量或者mikanani.me
fn generate_url(url: &str) -> Result<Url, Error> {
    let mut u = if !url.starts_with("http") {
        let url = format!("https://{}{}", MIKANANI_DOMAIN.to_string(), url);
        Url::parse(&url).map_err(|_| Error::ParseUrl { url })?
    } else {
        Url::parse(url).map_err(|_| Error::ParseUrl { url: url.into() })?
    };

    u.set_host(Some(&MIKANANI_DOMAIN.to_string()))
        .map_err(|_| Error::ParseUrl { url: url.into() })?;
    u.set_scheme("https")
        .map_err(|_| Error::ParseUrl { url: url.into() })?;

    Ok(u)
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

    #[snafu(display("Failed to fetch episode page {url}"))]
    FetchEpisodePage { source: reqwest::Error, url: String },

    #[snafu(display("Failed to parse episode page {url}"))]
    ParseEpisodePage { url: String },

    #[snafu(display("Failed to parse url {url}"))]
    ParseUrl { url: String },

    #[snafu(display("Failed to parse anime page {url} with error: {error}"))]
    ParseAnimePage { url: String, error: String },

    #[snafu(display("Failed to link database with error: {}", source))]
    LinkDatabase { source: redb::Error },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_feed() {
        let sub = get_feed("https://mikanani.me/RSS/MyBangumi?token=7eyqLval7M7aHnZ08QNtMAVbg%2fipV4sY5pAYcasKRBI%3d")
            .await
            .unwrap();
        dbg!(sub);
    }

    #[tokio::test]
    async fn parse_url() {
        let url = "https://mikanani.me/Home/Bangumi/3344#583";
        let url = Url::parse(url).unwrap();

        // get 3344
        let bangumi_id = url.path_segments().unwrap().last().unwrap();
        // get 583
        let subgroup_id = url.fragment().unwrap();
        dbg!(bangumi_id, subgroup_id);
    }

    #[tokio::test]
    async fn test_get_info_from_anime_page() {
        let _anime = get_info_from_anime_page("https://mikanani.me/Home/Bangumi/3344#583")
            .await
            .expect("Failed to fetch episode page");

        dbg!(_anime);
    }
}
