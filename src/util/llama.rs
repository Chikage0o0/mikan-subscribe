use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::reqwest::client;

static LLAMA: OnceLock<Llama> = OnceLock::new();

pub struct Llama {
    model: String,
    url: String,
    token: String,
}

impl Llama {
    pub fn get() -> Option<&'static Llama> {
        LLAMA.get()
    }

    pub fn init(model: &str, url: &str, token: &str) -> Result<(), String> {
        let llama = Llama {
            model: model.to_string(),
            url: url.to_string(),
            token: token.to_string(),
        };
        LLAMA
            .set(llama)
            .map_err(|_| "Llama has been initialized".to_string())?;

        Ok(())
    }

    pub async fn decode(&self, filename: &str) -> Result<ContentResponse, String> {
        let messages = vec![
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::System,
                content: Some(SYSTEM.to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some(
                    "[Billion Meta Lab] 恋语轻唱 Sasayaku You ni Koi wo Utau [02][1080][CHS].mp4"
                        .to_string(),
                ),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::Assistant,
                content: Some(r#"该文件名中的标题是 "恋语轻唱 Sasayaku You ni Koi wo Utau",但"恋语轻唱"与日本罗马音"Sasayaku You ni Koi wo Utau"所表达的意思相同，在这种情况下，优先选用由字母组成的标题，故使用"Sasayaku You ni Koi wo Utau"作为标题。没有明确的季数标识，通常默认为第1季，由于文件名中的"[02]"指示这是第2集，因此我们可以确定这是第2集。
{"title": "Sasayaku You ni Koi wo Utau", "season": 1, "episode": 2}"#.to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some("[Up to 21°C] 永生 第三季 - 06 (B-Global Donghua 1920x1080 HEVC AAC MKV) [9F7CAB79].mkv".to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::Assistant,
                content: Some(r#"该文件名中的标题是 "永生",由于有"第三季"的明确标识，表明这是第3季(season: 3)，而集数则由"06"指出，表示这是第6集(episode: 6)。
{"title": "永生", "season": 3, "episode": 6}"#.to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some("[GJ.Y] 我内心危险的东西2 - 05 (B-Global 1920x1080 HEVC AAC MKV) [60D8C635].mkv".to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::Assistant,
                content: Some(r#"从文件名可以解析出，主要名字为"我内心危险的东西2",由于末尾跟随数字，且该剧集其他地方都没有明确的季数标识，因此我们可以推断这是第2季(season: 2)，那么标题则为"我内心危险的东西"，而集数则是由"05"指出，表示这是第5集(episode: 5)。
{"title":"我内心危险的东西","season":2,"episode":5}"#.to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some("[Up to 21°C] Shinigami Bocchan to Kuro Maid 3rd Season - 30 (CR 1920x1080 AVC AAC MKV) [CE516DAA].mkv".to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::Assistant,
                content: Some(r#"根据文件名，动画标题为"Shinigami Bocchan to Kuro Maid"，由于有"3rd Season"的明确标识，表明这是第三季(season: 3)，而集数则由"30"指出，表示这是第30集(episode: 30)。
{"title":"Shinigami Bocchan to Kuro Maid","season":3,"episode":30}"#.to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some("[Sakurato] Dungeon Meshi [17v2][AVC-8bit 1080p AAC][CHT].mp4".to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::Assistant,
                content: Some(r#"根据文件名，动画标题为"Dungeon Meshi"，但由于文件名中没有直接提供季数信息，我们通常假设这是第一季(除非有其他已知信息指明它属于后续季)。集数可以从"17v2"推断，虽然这里的"v2"可能意味着这是一个修订版或版本2，但主要的集数识别还是"17"，因此是第17集。
{"title":"Dungeon Meshi","season":1,"episode":17}"#.to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some("[Up to 21°C] Mushoku Tensei II -  Isekai Ittara Honki Dasu Part 2 - 18 (ABEMA 1920x1080 AVC AAC MP4) [05628308].mp4".to_string()),
            },
            ChatCompletionMessage{
                role: ChatCompletionMessageRole::Assistant,
                content: Some(r#"根据文件名，动画标题为"Mushoku Tensei II - Isekai Ittara Honki Dasu"，注意到这里的"II"提示我们这是第二部或第二季(season: 2)。文件名中的"Part 2"可能指的是系列内部的另一个划分，但在解析到季节和集数时，最重要的是识别到它是第二季。集数直接从"18"读取，即第18集(episode: 18)
{"title":"Mushoku Tensei II - Isekai Ittara Honki Dasu","season":2,"episode":18}"#.to_string()),
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some(filename.to_string()),
            },
        ];

        let chat_req = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: 0.01,
        };

        let res = client()
            .post(&self.url)
            .bearer_auth(&self.token)
            .json(&chat_req)
            .send()
            .await
            .map_err(|e| format!("Request error: {}", e))?;

        let res = res
            .json::<Response>()
            .await
            .map_err(|e| format!("Json error: {}", e))?;

        let choice = res.choices.first().ok_or("No choice")?;
        let message = &choice.message.content;
        for line in message.lines() {
            // println!("{}", line);
            if let Ok(content) = serde_json::from_str::<ContentResponse>(line) {
                return Ok(content);
            }
        }

        Err("No content".to_string())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentResponse {
    title: String,
    season: u32,
    pub episode: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MessageResponse {
    role: ChatCompletionMessageRole,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChoiceResponse {
    index: usize,
    message: MessageResponse,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Response {
    choices: Vec<ChoiceResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatCompletionMessage>,
    temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatCompletionMessage {
    role: ChatCompletionMessageRole,
    content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ChatCompletionMessageRole {
    System,
    User,
    Assistant,
}

const SYSTEM: &str = r#"请将以下的动画文件名解析为包含"title"、"season"和"episode"三个键的JSON对象。"title"键的值应为动画的标题，"season"键的值应为动画的季数，"episode"键的值应为动画的集数。请确保你的解析结果准确无误。"#;

#[cfg(test)]
mod tests {
    use crate::util::{self, reqwest::init_client};

    use super::*;

    #[tokio::test]
    async fn test_decode() {
        let settings = util::config::Settings::load_from_file("settings.json").unwrap();
        let _ = init_client(settings.proxy).unwrap();
        if let Some(llama) = settings.llama {
            Llama::init(&llama.model, &llama.url, &llama.token).unwrap();
        }

        let filename =
            "[ANi] 她來自煩星（僅限港澳台地區） - 02 [1080P][Bilibili][WEB-DL][AAC AVC][CHT CHS].mp4";
        let content = Llama::get().unwrap().decode(filename).await.unwrap();
        assert_eq!(content.title, "她來自煩星");
        assert_eq!(content.season, 1);
        assert_eq!(content.episode, 2);
    }
}
